/* eslint-disable no-unsafe-finally */
/** Runs reproducible before/after pair benchmarks without changing the checkout or database. */
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { cpus, tmpdir } from 'node:os';
import { dirname, join, resolve, basename } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const reference = process.argv[2] ?? '2b2bb29';
const startedAt = new Date();
const timestamp = [
    startedAt.getFullYear(),
    startedAt.getMonth() + 1,
    startedAt.getDate(),
    startedAt.getHours(),
    startedAt.getMinutes()
]
    .map(value => String(value).padStart(2, '0'))
    .join('-');
const matcherPath = 'src-tauri/src/feature/duplicates/matcher.rs';
const driverPath = 'scripts/bench-duplicate-matching.rs';
const temporary = mkdtempSync(join(tmpdir(), 'mundam-matching-'));
const executable = join(temporary, process.platform === 'win32' ? 'benchmark.exe' : 'benchmark');
const driver = readFileSync(join(root, driverPath), 'utf8');
const platform = { win32: 'windows', darwin: 'macos' }[process.platform] ?? process.platform;
const outputDirectory = join(root, 'docs/backend/workers/benchmarks');
const prefix = join(outputDirectory, `${timestamp}-duplicate-matching-${platform}`);
const toolchain = process.platform === 'win32' ? '+stable-x86_64-pc-windows-msvc' : '+stable';

/** Compiles only the production math with optimizations; no Tauri or SQLite is linked. */
function measure(source, prepared) {
    const generated = join(temporary, 'benchmark.rs');
    writeFileSync(generated, source);
    const argumentsList = ['--edition', '2021', '-O', generated, '-o', executable];
    if (prepared) argumentsList.push('--cfg', 'prepared');
    execFileSync('rustc', [toolchain, ...argumentsList], { cwd: root, stdio: 'inherit' });
    return execFileSync(executable, [], { cwd: root, encoding: 'utf8' });
}

/** Reads measured rows independently of the human-readable comment and header. */
function rows(output) {
    return output
        .trim()
        .split(/\r?\n/)
        .filter(line => !line.startsWith('#'))
        .slice(1)
        .map(line => line.split('\t'));
}

try {
    if (
        ['-before.tsv', '-after.tsv', '-metadata.json'].some(suffix => existsSync(prefix + suffix))
    ) {
        throw new Error(
            'Benchmark results already exist for this start minute; keep the existing evidence and retry in another minute.'
        );
    }
    const baseline = join(temporary, 'matcher.rs');
    const revision = execFileSync('git', ['rev-parse', reference], {
        cwd: root,
        encoding: 'utf8'
    }).trim();
    writeFileSync(
        baseline,
        execFileSync('git', ['show', `${revision}:${matcherPath}`], { cwd: root })
    );
    const before = measure(
        driver.replace(`../${matcherPath}`, baseline.replaceAll('\\', '/')),
        false
    );
    const after = measure(
        driver.replace(`../${matcherPath}`, join(root, matcherPath).replaceAll('\\', '/')),
        true
    );
    const previous = rows(before);
    const current = rows(after);
    if (previous.length !== current.length) throw new Error('Benchmark scenario counts differ.');
    console.log('scenario\tassets\tbefore_ms\tafter_ms\tspeedup');
    current.forEach((row, index) => {
        const prior = previous[index];
        if ([0, 1, 2, 4].some(column => prior[column] !== row[column])) {
            throw new Error(`Matching result changed in scenario ${row[0]} (${row[1]} assets).`);
        }
        console.log(
            [
                row[0],
                row[1],
                Number(prior[3]) / 1000,
                Number(row[3]) / 1000,
                (Number(prior[3]) / Number(row[3])).toFixed(2)
            ].join('\t')
        );
    });
    mkdirSync(outputDirectory, { recursive: true });
    writeFileSync(`${prefix}-before.tsv`, before);
    writeFileSync(`${prefix}-after.tsv`, after);
    writeFileSync(
        `${prefix}-metadata.json`,
        JSON.stringify(
            {
                startedAt: startedAt.toISOString(),
                measuredAt: new Date().toISOString(),
                baselineRevision: revision,
                platform,
                arch: process.arch,
                processor: cpus()[0]?.model,
                compiler: execFileSync('rustc', [toolchain, '--version'], {
                    encoding: 'utf8'
                }).trim(),
                matcherSha256: createHash('sha256')
                    .update(readFileSync(join(root, matcherPath)))
                    .digest('hex'),
                driverSha256: createHash('sha256').update(driver).digest('hex'),
                scope: 'Synthetic pair evaluation, preparation included; excludes grouping, extraction, SQLite and UI.'
            },
            null,
            2
        ) + '\n'
    );
} finally {
    if (
        dirname(resolve(temporary)) !== resolve(tmpdir()) ||
        !basename(temporary).startsWith('mundam-matching-')
    ) {
        throw new Error('Unexpected benchmark temporary directory.');
    }
    rmSync(temporary, { recursive: true, force: true });
}
