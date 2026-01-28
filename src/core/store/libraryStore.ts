import { createStore, reconcile } from "solid-js/store";
import { getImages } from "../../lib/db";
import { tagService } from "../../lib/tags";
import { filterState, filterActions } from "./filterStore";


export interface ImageItem {
  id: number;
  path: string;
  filename: string;
  width: number | null;
  height: number | null;
  thumbnail_path: string | null;
  rating: number;
  notes: string | null;
  size: number;
  created_at: string;
  modified_at: string;
}

interface LibraryState {
  items: ImageItem[];
  isFetching: boolean;
  totalItems: number; // useful for knowing if we reached end
}

const BATCH_SIZE = 100;
let currentOffset = 0;

const [libraryState, setLibraryState] = createStore<LibraryState>({
  items: [],
  isFetching: false,
  totalItems: 0
});

export const libraryActions = {
  refreshImages: async (reset = false) => {

    const isUntagged = filterState.filterUntagged;
    const folderId = filterState.selectedFolderId;
    const recursive = filterState.folderRecursiveView;
    const anyFilter = filterActions.hasActiveFilters();
    
    console.log("libraryStore.refreshImages", { reset, isUntagged, folderId, recursive, anyFilter });

    if (reset) {
      currentOffset = 0;
      let firstBatch;
      if (anyFilter) {
        firstBatch = await tagService.getImagesFiltered(
          BATCH_SIZE, 0, filterState.selectedTags, true, isUntagged, folderId || undefined, recursive
        );
      } else {
        firstBatch = await getImages(BATCH_SIZE, 0);
      }
      setLibraryState("items", reconcile(firstBatch, { key: "id" }));
      currentOffset = BATCH_SIZE;
    } else {
      let fresh;
      if (anyFilter) {
        fresh = await tagService.getImagesFiltered(
          BATCH_SIZE, 0, filterState.selectedTags, true, isUntagged, folderId || undefined, recursive
        );
      } else {
        fresh = await getImages(BATCH_SIZE, 0);
      }
      setLibraryState("items", reconcile(fresh, { key: "id" }));
      currentOffset = BATCH_SIZE;
    }
  },

  loadMore: async () => {
    if (libraryState.isFetching) return;
    setLibraryState("isFetching", true);

    try {
      const isUntagged = filterState.filterUntagged;
      const folderId = filterState.selectedFolderId;
      const recursive = filterState.folderRecursiveView;
      const anyFilter = filterActions.hasActiveFilters();
      
      let nextBatch;
      
      if (anyFilter) {
        nextBatch = await tagService.getImagesFiltered(
          BATCH_SIZE, currentOffset, filterState.selectedTags, true, isUntagged, folderId || undefined, recursive
        );
      } else {
        nextBatch = await getImages(BATCH_SIZE, currentOffset);
      }

      if (nextBatch.length > 0) {
        setLibraryState("items", (prev) => [...prev, ...nextBatch]);
        currentOffset += BATCH_SIZE;
      }
    } finally {
      setLibraryState("isFetching", false);
    }
  },

  updateItemRating: async (id: number, rating: number) => {
    try {
      setLibraryState("items", i => i.id === id, "rating", rating);
      await tagService.updateImageRating(id, rating);
    } catch (err) {
      console.error(`Failed to update rating for ${id}:`, err);
    }
  },

  updateItemNotes: async (id: number, notes: string) => {
    try {
      setLibraryState("items", i => i.id === id, "notes", notes);
      await tagService.updateImageNotes(id, notes);
    } catch (err) {
      console.error(`Failed to update notes for ${id}:`, err);
    }
  },

  updateThumbnail: (id: number, path: string) => {
    setLibraryState(
      "items",
      (item) => item.id === id,
      "thumbnail_path",
      path
    );
  }
};

export { libraryState };
