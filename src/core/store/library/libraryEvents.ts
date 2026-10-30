import { libraryActions } from './libraryActions';
import { metadataActions } from '../metadata';
import type { BatchChangePayload } from './schemas';

/**
 * Applies one authoritative refresh for a committed folder-removal operation.
 * Ordinary watcher changes retain incremental updates and debounce behavior.
 *
 * @param payload - The backend library change envelope.
 * @returns Completion of the required store refreshes.
 */
export async function handleLibraryBatchChange(payload: BatchChangePayload): Promise<void> {
    if (payload.operation === 'folder_removal') {
        await Promise.all([
            libraryActions.refreshAfterFolderRemoval(),
            metadataActions.refreshAfterFolderRemoval()
        ]);
        return;
    }
    libraryActions.handleBatchChange(payload);
    metadataActions.handleBatchChange(payload);
}
