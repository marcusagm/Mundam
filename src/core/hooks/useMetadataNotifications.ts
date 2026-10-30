import { onCleanup, onMount } from 'solid-js';
import { listen } from '@tauri-apps/api/event';
import { useNotification } from './useNotification';
import { type BatchChangePayload } from '../store/library';

/**
 * Counts entries in an optional backend change list.
 *
 * @param changes - Changed assets supplied by the event.
 * @returns The list size, or zero when the list is absent.
 */
function countChanges(changes: readonly unknown[] | undefined): number {
    return changes?.length || 0;
}

/**
 * Hook that listens for background library changes and triggers notifications.
 * This decouples core state management from transient UI feedback (toasts).
 */
export const useMetadataNotifications = () => {
    const notification = useNotification();

    onMount(() => {
        let unlistenBatchChange: (() => void) | null = null;

        // Set up listener for library batch changes (emitted by backend/indexing)
        listen<BatchChangePayload>('library:batch-change', event => {
            const payload = event.payload;
            if (payload.operation === 'folder_removal') return;
            const addedCount = countChanges(payload.added);
            const removedCount = countChanges(payload.removed);
            const updatedCount = countChanges(payload.updated);

            if (addedCount > 0) {
                notification.success(
                    'Library Sync',
                    addedCount === 1 ? '1 asset added' : `${addedCount} assets added`
                );
            }

            if (removedCount > 0) {
                notification.info(
                    'Library Sync',
                    removedCount === 1 ? '1 asset removed' : `${removedCount} assets removed`
                );
            }

            if (updatedCount > 0) {
                notification.info(
                    'Library Sync',
                    updatedCount === 1 ? '1 asset updated' : `${updatedCount} assets updated`
                );
            }
        }).then(unlisten => {
            unlistenBatchChange = unlisten;
        });

        onCleanup(() => {
            if (unlistenBatchChange) unlistenBatchChange();
        });
    });
};
