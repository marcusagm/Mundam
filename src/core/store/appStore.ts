import { createStore, reconcile } from "solid-js/store";
import { createSignal } from "solid-js";
import { getImages, getLocations, initDb, addLocation } from "../../lib/db";
import { tagService, Tag } from "../../lib/tags";
import { tauriService } from "../tauri/services";
import { listen } from "@tauri-apps/api/event";

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

export interface ProgressPayload {
  total: number;
  processed: number;
  current_file: string;
}

// Global Store State
interface AppState {
  items: ImageItem[];
  locations: { id: number; path: string; name: string }[];
  selection: number[]; // ID of selected images
  tags: Tag[]; 
  selectedTags: number[]; // ID of selected tags for filtering
  selectedLocationId: number | null;
  filterUntagged: boolean;
  libraryStats: {
      total_images: number;
      untagged_images: number;
      tag_counts: Map<number, number>;
      folder_counts: Map<number, number>;
  };
}

// Reactive primitives
const [state, setState] = createStore<AppState>({ 
  items: [], 
  locations: [], 
  selection: [],
  tags: [],
  selectedTags: [],
  selectedLocationId: null,
  filterUntagged: false,
  libraryStats: {
      total_images: 0,
      untagged_images: 0,
      tag_counts: new Map(),
      folder_counts: new Map(),
  }
});
const [rootPath, setRootPath] = createSignal<string | null>(null);
const [loading, setLoading] = createSignal(true);
const [progress, setProgress] = createSignal<ProgressPayload | null>(null);
const [searchQuery, setSearchQuery] = createSignal("");
const [tagUpdateTrigger, setTagUpdateTrigger] = createSignal(0);

// Pagination State (Module Level or inside Store if reactive needed)
let currentOffset = 0;
let isFetching = false;
const BATCH_SIZE = 100;

// Actions
export const appActions = {
  notifyTagUpdate: () => {
      setTagUpdateTrigger(n => n + 1);
      appActions.loadLibraryStats();
      
      // If any relevant filter is active, refresh the list immediately
      const anyFilter = state.selectedTags.length > 0 || state.filterUntagged;
      if (anyFilter) {
          appActions.refreshImages(true);
      }
  },

  loadLibraryStats: async () => {
      try {
          const stats = await tagService.getLibraryStats();
          const tagMap = new Map();
          stats.tag_counts.forEach(c => tagMap.set(c.tag_id, c.count));
          
          const folderMap = new Map();
          stats.folder_counts.forEach(c => folderMap.set(c.location_id, c.count));

          setState("libraryStats", {
              total_images: stats.total_images,
              untagged_images: stats.untagged_images,
              tag_counts: tagMap,
              folder_counts: folderMap
          });
      } catch (err) {
          console.error("Failed to load library stats:", err);
      }
  },

  initialize: async () => {
    try {
      setLoading(true);
      await initDb();
      const locations = await getLocations();
      setState("locations", locations);
      
      // Load Tags & Stats
      await Promise.all([
          appActions.loadTags(),
          appActions.loadLibraryStats()
      ]);
      
      if (locations.length > 0) {
        const path = locations[0].path;
        setRootPath(path);
        
        // Start indexing in background
        tauriService.startIndexing({ path }).catch(console.error);
      }

      // Initial Load
      await appActions.refreshImages(true);
      
      // Setup listeners
      appActions.setupListeners();
    } catch (err) {
      console.error("Initialization failed:", err);
    } finally {
      setLoading(false);
    }
  },

  loadTags: async () => {
      try {
          const tags = await tagService.getAllTags();
          setState("tags", tags);
      } catch (err) {
          console.error("Failed to load tags:", err);
      }
  },

  refreshImages: async (reset = false) => {
    const hasTagFilter = state.selectedTags.length > 0;
    const isUntagged = state.filterUntagged;
    const locationId = state.selectedLocationId;
    const anyFilter = hasTagFilter || isUntagged || locationId !== null;
    
    if (reset) {
        currentOffset = 0;
        let firstBatch;
        if (anyFilter) {
            firstBatch = await tagService.getImagesFiltered(
                BATCH_SIZE, 0, state.selectedTags, true, isUntagged, locationId || undefined
            );
        } else {
            firstBatch = await getImages(BATCH_SIZE, 0);
        }
        setState("items", reconcile(firstBatch, { key: "id" }));
        currentOffset = BATCH_SIZE;
    } else {
        // Partial re-fetch usually for progress or small updates
        let fresh;
        if (anyFilter) {
            fresh = await tagService.getImagesFiltered(
                BATCH_SIZE, 0, state.selectedTags, true, isUntagged, locationId || undefined
            );
        } else {
            fresh = await getImages(BATCH_SIZE, 0);
        }
        setState("items", reconcile(fresh, { key: "id" }));
        currentOffset = BATCH_SIZE;
    }
  },

  loadMore: async () => {
    if (isFetching) return;
    isFetching = true;

    try {
        const hasTagFilter = state.selectedTags.length > 0;
        const isUntagged = state.filterUntagged;
        const locationId = state.selectedLocationId;
        const anyFilter = hasTagFilter || isUntagged || locationId !== null;
        
        let nextBatch;
        
        if (anyFilter) {
            nextBatch = await tagService.getImagesFiltered(
                BATCH_SIZE, currentOffset, state.selectedTags, true, isUntagged, locationId || undefined
            );
        } else {
            nextBatch = await getImages(BATCH_SIZE, currentOffset);
        }

        if (nextBatch.length > 0) {
            setState("items", (prev) => [...prev, ...nextBatch]);
            currentOffset += BATCH_SIZE;
        }
    } finally {
        isFetching = false;
    }
  },

  setRootLocation: async (path: string, name: string) => {
    await addLocation(path, name);
    // Reload locations to get the ID and valid state
    const locations = await getLocations();
    setState("locations", locations);
    setRootPath(path);
    await tauriService.startIndexing({ path });
  },

  toggleSelection: (id: number, multi: boolean) => {
    if (multi) {
      const current = state.selection;
      if (current.includes(id)) {
        setState("selection", current.filter(i => i !== id));
      } else {
        setState("selection", [...current, id]);
      }
    } else {
      setState("selection", [id]);
    }
  },
  
  toggleTagSelection: (tagId: number) => {
      const current = state.selectedTags;
      if (current.includes(tagId)) {
          setState("selectedTags", (tags) => tags.filter(id => id !== tagId));
      } else {
          setState({
              selectedTags: [...current, tagId],
              filterUntagged: false // Clear untagged when selecting tags
          });
      }
      appActions.refreshImages(true);
  },

  toggleUntagged: () => {
      setState("filterUntagged", (v) => !v);
      if (state.filterUntagged) {
          // If we want untagged, tags filter doesn't make sense
          setState("selectedTags", []);
      }
      appActions.refreshImages(true);
  },

  selectLocation: (id: number | null) => {
      setState("selectedLocationId", id);
      appActions.refreshImages(true);
  },

  clearAllFilters: () => {
      setState({
          selectedTags: [],
          selectedLocationId: null,
          filterUntagged: false
      });
      appActions.refreshImages(true);
  },

  setSearch: (query: string) => {
    setSearchQuery(query);
  },

  setupListeners: async () => {
    // Indexer Progress
    await listen<ProgressPayload>("indexer:progress", (event) => {
      setProgress(event.payload);
      if (event.payload.processed % 10 === 0 || event.payload.processed === event.payload.total) {
        appActions.refreshImages();
      }
    });

    // Indexer Complete
    await listen<number>("indexer:complete", async (event) => {
      console.log("Indexer complete. Total:", event.payload);
      setProgress(null);
      await Promise.all([
          appActions.loadLibraryStats(),
          appActions.refreshImages(true)
      ]);
    });

    // Thumbnail Generation Updates
    await listen<{ id: number; path: string }>("thumbnail:ready", (event) => {
        setState(
            "items",
            (item) => item.id === event.payload.id,
            "thumbnail_path",
            event.payload.path
        );
    });
  },

  updateItemRating: async (id: number, rating: number) => {
      try {
          // Optimistic update
          setState("items", i => i.id === id, "rating", rating);
          await tagService.updateImageRating(id, rating);
      } catch (err) {
          console.error(`Failed to update rating for ${id}:`, err);
          // Revert could be implemented here if strictly needed
      }
  },

  updateItemNotes: async (id: number, notes: string) => {
      try {
          // Optimistic update
          setState("items", i => i.id === id, "notes", notes);
          await tagService.updateImageNotes(id, notes);
      } catch (err) {
           console.error(`Failed to update notes for ${id}:`, err);
      }
  },

  setSelection: (ids: number[]) => {
      setState("selection", ids);
  }
};

// Exports for consumption
export const useAppStore = () => {
  return {
    state, // Read-only store (mostly)
    rootPath,
    loading,
    progress,
    searchQuery,
    tagUpdateTrigger
  };
};
