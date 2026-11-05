import { executeLibraryOperation } from '../store/library/folderActions';
import { createMemo } from 'solid-js';
import { libraryState, libraryActions } from '../store/library';

const itemsCountMemo = createMemo(() => libraryState.items.length);

/**
 * Hook providing access to library state and actions.
 *
 * @returns {Object} Library state accessors and refinement methods.
 */
export const useLibrary = () => {
    return {
        // State
        get items() {
            return libraryState.items;
        },
        get isFetching() {
            return libraryState.isFetching;
        },
        get totalItems() {
            return libraryState.totalItems;
        },
        /** Memoized count of items currently loaded in memory */
        loadedCount: itemsCountMemo,

        // Actions
        refreshAssets: libraryActions.refreshAssets,
        loadMore: libraryActions.loadMore,
        updateItemRating: libraryActions.updateItemRating,
        updateItemNotes: libraryActions.updateItemNotes,
        toggleItemFavorite: libraryActions.toggleItemFavorite,
        moveToTrashAssets: libraryActions.moveToTrashAssets,
        restoreFromTrashAssets: libraryActions.restoreFromTrashAssets,
        updateThumbnail: libraryActions.updateThumbnail,
        setThumbnailPriority: libraryActions.setThumbnailPriority,
        executeOperation: executeLibraryOperation,
        addLocation: libraryActions.addLocation,
        removeLocation: libraryActions.removeLocation
    };
};
