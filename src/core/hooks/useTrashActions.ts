/**
 * useTrashActions
 *
 * Single entry point for moving assets to the trash, including the
 * success notification with an "Undo" action.
 */

import { useLibrary } from './useLibrary';
import { useNotification } from './useNotification';

export interface TrashActions {
    /** Move the given asset ids to the trash and notify the user (with undo). */
    moveToTrash: (assetIds: string[]) => Promise<void>;
}

export function useTrashActions(): TrashActions {
    const library = useLibrary();
    const notify = useNotification();

    return {
        moveToTrash: async (assetIds: string[]) => {
            if (assetIds.length === 0) return;

            await library.moveToTrashAssets(assetIds);

            notify.success(
                assetIds.length > 1
                    ? `${assetIds.length} items moved to trash`
                    : 'Item moved to trash',
                undefined,
                {
                    label: 'Undo',
                    onClick: async () => {
                        await library.restoreFromTrashAssets(assetIds);
                        notify.success('Action undone', 'Items have been restored');
                    }
                }
            );
        }
    };
}
