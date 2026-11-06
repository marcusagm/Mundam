/** Keeps the audited library boundary strict without hiding unrelated repository debt. */
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import ts from 'typescript';

const frontend = [
    'src/components/features/library',
    'src/components/features/settings/FolderRecoverySettings.tsx',
    'src/components/features/settings/TrashSettingsSection.tsx',
    'src/components/features/tags/TagContextMenu.tsx',
    'src/components/features/tags/TagTreeSidebarPanel.tsx',
    'src/components/ui/ContextMenu',
    'src/components/ui/TreeView',
    'src/core/dnd/createTreeDragItem.ts',
    'src/core/dnd/parseDragItem.ts',
    'src/core/store/library/folderActions.ts',
    'src/core/store/metadata/tagActions.ts',
    'src/lib/tags.ts'
];
const backend = [
    'src-tauri/src/core/library',
    'src-tauri/src/feature/library/operations.rs',
    'src-tauri/src/infra/database/handlers/library',
    'src-tauri/src/infra/database/handlers/folder_handler.rs',
    'src-tauri/src/infra/database/query_handlers/folder_queries.rs',
    'src-tauri/src/infra/database/query_handlers/library_identity.rs',
    'src-tauri/src/infra/database/query_handlers/library_receipts.rs',
    'src-tauri/src/infra/database/query_handlers/library_recovery.rs',
    'src-tauri/src/infra/database/query_handlers/path_identity.rs',
    'src-tauri/src/infra/database/library_operations.rs',
    'src-tauri/src/infra/database/library_operations/journal.rs',
    'src-tauri/src/infra/filesystem/identity.rs'
];

function files(entry) {
    if (!fs.statSync(entry).isDirectory()) return [entry];
    return fs.readdirSync(entry).flatMap(child => files(path.join(entry, child)));
}

let failed = false;
function report(file, message) {
    console.error(`${file}: ${message}`);
    failed = true;
}

const documentedFrontend = [
    'src/components/features/library/FolderActionsDialog.tsx',
    'src/components/features/library/FolderDestinationPicker.tsx',
    'src/components/features/library/useFolderTreeActions.ts',
    'src/components/features/library/folderDeletionUndo.ts',
    'src/components/features/library/folderHierarchy.ts',
    'src/components/features/library/folderPresentation.ts',
    'src/components/ui/ContextMenu/ContextMenuCheckboxItem.tsx',
    'src/core/store/library/folderActions.ts',
    'src/core/dnd/createTreeDragItem.ts',
    'src/core/dnd/parseDragItem.ts'
];
for (const file of documentedFrontend) {
    const source = ts.createSourceFile(
        file,
        fs.readFileSync(file, 'utf8'),
        ts.ScriptTarget.Latest,
        true
    );
    for (const statement of source.statements) {
        if (!statement.modifiers?.some(modifier => modifier.kind === ts.SyntaxKind.ExportKeyword))
            continue;
        const callable =
            ts.isFunctionDeclaration(statement) ||
            (ts.isVariableStatement(statement) &&
                statement.declarationList.declarations.some(
                    declaration =>
                        declaration.initializer && ts.isArrowFunction(declaration.initializer)
                ));
        if (!callable) continue;
        const tags = ts.getJSDocTags(statement).map(tag => tag.tagName.text);
        if (!tags.includes('returns'))
            report(file, 'Exported function/component requires @returns.');
        const parameters = ts.isFunctionDeclaration(statement)
            ? statement.parameters
            : statement.declarationList.declarations[0]?.initializer?.parameters;
        if (parameters?.length && !tags.includes('param'))
            report(file, 'Exported parameters require @param.');
    }
}
for (const file of backend.flatMap(files).filter(file => file.endsWith('.rs'))) {
    const source = fs.readFileSync(file, 'utf8');
    for (const match of source.matchAll(/pub(?:\([^)]*\))?\s+(?:async\s+)?fn\s+\w+/g)) {
        const signature = source.slice(match.index, source.indexOf('{', match.index));
        if (!signature.includes('AppResult<')) continue;
        const prefix = source.slice(0, match.index);
        const documentation =
            prefix
                .replace(/(?:[ \t]*#\[[^\n]*\]\r?\n)+[ \t]*$/, '')
                .match(/(?:[ \t]*\/\/\/[^\n]*\r?\n)+[ \t]*$/)?.[0] ?? '';
        if (!documentation.includes('# Errors'))
            report(file, `${match[0]} requires Rustdoc # Errors.`);
    }
}
const result = spawnSync(
    process.execPath,
    ['node_modules/eslint/bin/eslint.js', ...frontend, '--ext', '.ts,.tsx', '--max-warnings', '0'],
    { stdio: 'inherit' }
);
if (result.status !== 0) failed = true;
process.exitCode = failed ? 1 : 0;
