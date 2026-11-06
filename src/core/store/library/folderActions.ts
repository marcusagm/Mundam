import { invokeCommand } from '../../../lib/api';
import { metadataActions, metadataState } from '../metadata';
import { libraryActions } from './libraryActions';
import { filterActions, filterState } from '../filter';

/** User intentions supported by the managed filesystem command. */
export type LibraryOperation =
    | { kind: 'createFolder'; parentId: string; name: string }
    | { kind: 'cancelFolderDraft'; folderId: string; creationOperationId: string }
    | { kind: 'renameFolder'; folderId: string; name: string }
    | { kind: 'moveFolder'; folderId: string; destinationId: string }
    | { kind: 'deleteFolder'; folderId: string }
    | { kind: 'restoreFolder'; folderId: string }
    | { kind: 'folderColor'; folderId: string; color: string | null; inheritsColor: boolean }
    | { kind: 'renameAsset'; assetId: string; name: string }
    | { kind: 'moveAssets'; assetIds: string[]; destinationId: string };

/** Authoritative identity and optional warning after a committed disk edit. */
export interface LibraryOperationResult {
    /** Durable receipt for tracing and ownership checks. */
    operationId: string;
    /** Catalog entity affected by this intention. */
    id: string;
    /** Discriminator independent of asset DTO fields. */
    entityType: 'folder' | 'asset';
    /** Confirmed destination on disk. */
    path: string;
    /** Actual source, absent for presentation-only changes. */
    previousPath?: string | null;
    /** Confirmed movements; intermediate case-rename paths are omitted. */
    effects: { source: string | null; destination: string | null }[];
    /** Retryable monitoring or refresh issue after a confirmed write. */
    warning?: string | null;
}

/** Commits an intention, then refreshes views without misreporting a committed write.
 * @param operation - Explicit backend intention.
 * @returns Confirmed receipt; warnings describe retryable monitoring/refresh failures.
 * @throws IPC validation or storage errors before commit.
 * @example await executeLibraryOperation({ kind: 'restoreFolder', folderId: 'folder-id' });
 */
export async function executeLibraryOperation(
    operation: LibraryOperation
): Promise<LibraryOperationResult> {
    const result = await invokeCommand<LibraryOperationResult>('library_operation', { operation });
    const refreshResults = await Promise.allSettled([
        metadataActions.loadLocations(),
        metadataActions.loadStats(),
        libraryActions.refreshAssets(false)
    ]);
    if (operation.kind === 'deleteFolder' || operation.kind === 'cancelFolderDraft') {
        if (
            filterState.selectedFolderIds.some(
                identifier => !metadataState.locations.some(folder => folder.id === identifier)
            )
        )
            filterActions.clearFolders();
    }
    for (const refresh of refreshResults) {
        if (refresh.status === 'rejected')
            console.warn('Library operation committed; view refresh failed:', refresh.reason);
    }
    if (refreshResults.some(refresh => refresh.status === 'rejected')) {
        result.warning = [
            result.warning,
            'Library updated. Refresh the view to reload current data.'
        ]
            .filter(Boolean)
            .join(' ');
    }
    return result;
}
