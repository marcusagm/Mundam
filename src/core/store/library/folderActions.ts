import { invokeCommand } from '../../../lib/api';
import { metadataActions, metadataState } from '../metadata';
import { libraryActions } from './libraryActions';
import { filterActions, filterState } from '../filter';

/** User intentions supported by the managed filesystem command. */
export type LibraryOperation =
    | { kind: 'createFolder'; parentId: string; name: string }
    | { kind: 'renameFolder'; folderId: string; name: string }
    | { kind: 'moveFolder'; folderId: string; destinationId: string }
    | { kind: 'deleteFolder'; folderId: string }
    | { kind: 'restoreFolder'; folderId: string }
    | { kind: 'folderColor'; folderId: string; color: string | null; inheritsColor: boolean }
    | { kind: 'renameAsset'; assetId: string; name: string }
    | { kind: 'moveAssets'; assetIds: string[]; destinationId: string };

/** Authoritative identity and optional warning after a committed disk edit. */
export interface LibraryOperationResult {
    id: string;
    path: string;
    warning?: string | null;
}

/** Commits a disk operation before reloading authoritative library metadata. */
export async function executeLibraryOperation(
    operation: LibraryOperation
): Promise<LibraryOperationResult> {
    const result = await invokeCommand<LibraryOperationResult>('library_operation', { operation });
    if (operation.kind === 'deleteFolder') {
        await metadataActions.loadLocations();
        if (
            filterState.selectedFolderIds.some(
                identifier => !metadataState.locations.some(folder => folder.id === identifier)
            )
        )
            filterActions.clearFolders();
    }
    const refreshResults = await Promise.allSettled([
        metadataActions.loadLocations(),
        metadataActions.loadStats(),
        libraryActions.refreshAssets(false)
    ]);
    for (const refresh of refreshResults) {
        if (refresh.status === 'rejected')
            console.warn('Library operation committed; view refresh failed:', refresh.reason);
    }
    return result;
}
