import { createStore, reconcile } from "solid-js/store";
import { getImages } from "../../lib/db";
import { tagService } from "../../lib/tags";
import { filterState, filterActions } from "./filterStore";


import { type ImageItem } from "../../types";

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
    const sortBy = filterState.sortBy;
    const sortOrder = filterState.sortOrder;
    
    const advancedQuery = filterState.advancedSearch ? JSON.stringify(filterState.advancedSearch) : undefined;
    
    console.log("libraryStore.refreshImages", { reset, isUntagged, folderId, recursive, anyFilter, sortBy, sortOrder, advancedQuery });

    if (reset) {
      currentOffset = 0;
      let firstBatch;
      if (anyFilter) {
        firstBatch = await tagService.getImagesFiltered(
          BATCH_SIZE, 0, filterState.selectedTags, true, isUntagged, folderId || undefined, recursive, sortBy, sortOrder, advancedQuery, filterState.searchQuery
        );
      } else {
        firstBatch = await getImages(BATCH_SIZE, 0, sortBy, sortOrder);
      }
      setLibraryState("items", reconcile(firstBatch, { key: "id" }));
      currentOffset = BATCH_SIZE;
    } else {
      let fresh;
      if (anyFilter) {
        fresh = await tagService.getImagesFiltered(
          BATCH_SIZE, 0, filterState.selectedTags, true, isUntagged, folderId || undefined, recursive, sortBy, sortOrder, advancedQuery, filterState.searchQuery
        );
      } else {
        fresh = await getImages(BATCH_SIZE, 0, sortBy, sortOrder);
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
      const sortBy = filterState.sortBy;
      const sortOrder = filterState.sortOrder;
      
      let nextBatch;
      const advancedQuery = filterState.advancedSearch ? JSON.stringify(filterState.advancedSearch) : undefined;
      
      if (anyFilter) {
        nextBatch = await tagService.getImagesFiltered(
          BATCH_SIZE, currentOffset, filterState.selectedTags, true, isUntagged, folderId || undefined, recursive, sortBy, sortOrder, advancedQuery, filterState.searchQuery
        );
      } else {
        nextBatch = await getImages(BATCH_SIZE, currentOffset, sortBy, sortOrder);
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
  },

  handleBatchChange: (payload: any) => {
      // 1. Handle Removals
      if (payload.removed && payload.removed.length > 0) {
          const removedIds = new Set(payload.removed.map((r: any) => r.id));
          setLibraryState("items", (items) => items.filter(i => !removedIds.has(i.id)));
      }

      // 2. Handle Additions
      if (payload.added && payload.added.length > 0) {
          // Trigger a soft refresh to integrate new items in the correct order/position
          // reconcile will handle merging existing ones
          libraryActions.refreshImages(false);
      }

      // 3. Handle Updates (Moves and Renames)
      if (payload.updated && payload.updated.length > 0) {
          import("./metadataStore").then(({ metadataState }) => {
              const selectedFolderId = filterState.selectedFolderId;
              const recursive = filterState.folderRecursiveView;

              const isChildOf = (childId: number, rootId: number): boolean => {
                  let current: number | null = childId;
                  while (current) {
                      if (current === rootId) return true;
                      const node = metadataState.locations.find(l => l.id === current);
                      current = node ? node.parent_id : null;
                  }
                  return false;
              };

              let someMovedIn = false;
              const toRemoveIDs: number[] = [];

              for (const item of payload.updated) {
                  const isNowInView = !selectedFolderId || 
                      (recursive ? isChildOf(item.folder_id, selectedFolderId) : item.folder_id === selectedFolderId);

                  const wasKnown = libraryState.items.some(i => i.id === item.id);

                  if (isNowInView) {
                      if (wasKnown) {
                          // Update in place (Rename or Move within same recursive tree)
                          setLibraryState("items", i => i.id === item.id, (prev) => ({
                              ...prev,
                              path: item.path,
                              filename: item.filename,
                              modified_at: item.modified_at,
                              folder_id: item.folder_id
                          }));
                      } else {
                          // Moved INTO this folder view from outside
                          someMovedIn = true;
                      }
                  } else if (wasKnown) {
                      // Was here, but moved OUT
                      toRemoveIDs.push(item.id);
                  }
              }

              if (toRemoveIDs.length > 0) {
                  const removeSet = new Set(toRemoveIDs);
                  setLibraryState("items", (items) => items.filter(i => !removeSet.has(i.id)));
              }

              if (someMovedIn) {
                  // Re-fetch to get items moved in
                  libraryActions.refreshImages(false);
              }
          });
      }
  }


};

export { libraryState };
