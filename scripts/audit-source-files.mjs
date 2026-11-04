/** Reports source files unreachable from application, test or Rust module entry points. */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import ts from 'typescript';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const relative = file => path.relative(root, file).replaceAll('\\', '/');

/** Inventories source only; dependencies and build output are outside the scanned roots. */
function files(directory) {
    return fs.readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
        const file = path.join(directory, entry.name);
        return entry.isDirectory() ? files(file) : [file];
    });
}

const frontend = files(path.join(root, 'src')).filter(file => /\.(tsx?|css)$/.test(file));
const known = new Set(frontend);
const edges = new Map();
const unresolved = [];

/** Resolves project-local imports, including barrel and stylesheet references. */
function resolveImport(specifier, source) {
    specifier = specifier.split('?')[0];
    if (!specifier.startsWith('.') && !specifier.startsWith('@/')) return;
    const candidate = specifier.startsWith('@/')
        ? path.join(root, 'src', specifier.slice(2))
        : path.resolve(path.dirname(source), specifier);
    const resolved = [
        candidate,
        ...['.ts', '.tsx', '.css', '/index.ts', '/index.tsx'].map(suffix => candidate + suffix)
    ]
        .map(file => path.normalize(file))
        .find(file => known.has(file));
    if (!resolved && /\.(tsx?|css)$/.test(specifier))
        unresolved.push(`${relative(source)} -> ${specifier}`);
    return resolved;
}

for (const file of frontend) {
    const text = fs.readFileSync(file, 'utf8');
    const imports = file.endsWith('.css')
        ? Array.from(text.matchAll(/@import\s+['"]([^'"]+)['"]/g), match => match[1])
        : ts.preProcessFile(text, true, true).importedFiles.map(item => item.fileName);
    if (!file.endsWith('.css')) {
        const ast = ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true);
        const visit = node => {
            if (
                ts.isNewExpression(node) &&
                node.expression.getText(ast) === 'URL' &&
                node.arguments?.length
            ) {
                const argument = node.arguments[0];
                if (ts.isStringLiteral(argument)) imports.push(argument.text);
            }
            if (
                ts.isCallExpression(node) &&
                (node.expression.kind === ts.SyntaxKind.ImportKeyword ||
                    node.expression.getText(ast) === 'vi.mock')
            ) {
                const argument = node.arguments[0];
                if (argument && ts.isStringLiteral(argument)) imports.push(argument.text);
            }
            ts.forEachChild(node, visit);
        };
        visit(ast);
    }
    edges.set(file, imports.map(specifier => resolveImport(specifier, file)).filter(Boolean));
}

/** Traverses the graph conservatively: imported barrel exports also count as referenced. */
function reachable(entries, graph) {
    const visited = new Set();
    const pending = [...entries];
    while (pending.length) {
        const file = pending.pop();
        if (visited.has(file)) continue;
        visited.add(file);
        pending.push(...(graph.get(file) ?? []));
    }
    return visited;
}

const entries = ['index.html', 'splashscreen.html'].flatMap(file =>
    Array.from(
        fs
            .readFileSync(path.join(root, file), 'utf8')
            .matchAll(/<script[^>]*src="\/(src\/[^"?]+)"/g),
        match => path.join(root, match[1])
    )
);
const production = reachable(entries, edges);
const tests = reachable(
    frontend.filter(file => /\.(spec|test)\.tsx?$/.test(file)),
    edges
);
const publicEntries = [
    'src/components/features/statusbar/index.ts',
    'src/core/input/primitives/index.ts',
    'src/core/input/providers/index.ts',
    'src/core/input/store/index.ts'
].map(file => path.join(root, file));
const publicApi = reachable(publicEntries, edges);
const candidates = frontend.filter(
    file =>
        !file.endsWith('.d.ts') && !production.has(file) && !tests.has(file) && !publicApi.has(file)
);

const rust = files(path.join(root, 'src-tauri/src')).filter(file => file.endsWith('.rs'));
const rustEdges = new Map();
const rustUnresolved = [];
for (const file of rust) {
    const text = fs.readFileSync(file, 'utf8');
    const directory = path.dirname(file);
    const childDirectory = /^(lib|main|mod)\.rs$/.test(path.basename(file))
        ? directory
        : file.slice(0, -3);
    let prior = 0;
    const references = [];
    for (const match of text.matchAll(/^(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*;/gm)) {
        const attributes = text.slice(prior, match.index);
        const explicit = Array.from(attributes.matchAll(/#\[path\s*=\s*"([^"]+)"\]/g)).at(-1)?.[1];
        const alternatives = explicit
            ? [path.resolve(directory, explicit)]
            : [
                  path.join(childDirectory, match[1] + '.rs'),
                  path.join(childDirectory, match[1], 'mod.rs')
              ];
        const resolved = alternatives.find(candidate => fs.existsSync(candidate));
        if (resolved) references.push(resolved);
        else rustUnresolved.push(`${relative(file)} -> ${match[1]}`);
        prior = match.index + match[0].length;
    }
    rustEdges.set(file, references);
}
const rustReachable = reachable(
    ['lib.rs', 'main.rs'].map(file => path.join(root, 'src-tauri/src', file)),
    rustEdges
);
const report = {
    scope: 'Conservative file reachability; includes tests, CSS and Rust cfg modules. Not an unused-export or runtime liveness proof.',
    frontend: {
        total: frontend.length,
        production: production.size,
        testOnly: [...tests].filter(file => !production.has(file)).length,
        publicOnly: [...publicApi]
            .filter(file => !production.has(file) && !tests.has(file))
            .map(relative),
        candidates: candidates.map(relative),
        unresolved
    },
    rust: {
        total: rust.length,
        candidates: rust.filter(file => !rustReachable.has(file)).map(relative),
        unresolved: rustUnresolved
    }
};
console.log(JSON.stringify(report, null, 2));

if (
    process.argv.includes('--strict') &&
    (report.frontend.candidates.length ||
        report.frontend.unresolved.length ||
        report.rust.candidates.length ||
        report.rust.unresolved.length)
) {
    process.exitCode = 1;
}
