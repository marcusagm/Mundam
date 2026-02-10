import { libraryState, libraryActions } from "../store/libraryStore";

export const useLibrary = () => {
  return {
    // State
    get items() { return libraryState.items; },
    get isFetching() { return libraryState.isFetching; },
    get totalItems() { return libraryState.totalItems; },

    // Actions
    refreshImages: libraryActions.refreshImages,
    loadMore: libraryActions.loadMore,
    updateItemRating: libraryActions.updateItemRating,
    updateItemNotes: libraryActions.updateItemNotes,
    updateThumbnail: libraryActions.updateThumbnail,
    setThumbnailPriority: libraryActions.setThumbnailPriority
  };
};
