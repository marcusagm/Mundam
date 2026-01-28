import { createStore } from "solid-js/store";
import { Tag, tagService } from "../../lib/tags";
import { getLocations } from "../../lib/db";

interface FolderNode {
  id: number;
  path: string;
  name: string;
  parent_id: number | null;
  is_root: boolean;
}

interface MetadataState {
  tags: Tag[];
  locations: FolderNode[];
  libraryStats: {
    total_images: number;
    untagged_images: number;
    tag_counts: Map<number, number>;
    folder_counts: Map<number, number>;
    folder_counts_recursive: Map<number, number>;
  };
  tagUpdateVersion: number;
}

const [metadataState, setMetadataState] = createStore<MetadataState>({
  tags: [],
  locations: [],
  libraryStats: {
    total_images: 0,
    untagged_images: 0,
    tag_counts: new Map(),
    folder_counts: new Map(),
    folder_counts_recursive: new Map(),
  },
  tagUpdateVersion: 0
});

export const metadataActions = {
  // ... (notifyTagUpdate same)
  notifyTagUpdate: () => {
    setMetadataState("tagUpdateVersion", v => v + 1);
    metadataActions.loadStats();
    
    // Check if we need to refresh the library
    import("./filterStore").then(({ filterState }) => {
        if (filterState.filterUntagged || filterState.selectedTags.length > 0) {
            import("./libraryStore").then(({ libraryActions }) => {
                libraryActions.refreshImages(true);
            });
        }
    });
  },

  loadTags: async () => {
    try {
      const tags = await tagService.getAllTags();
      setMetadataState("tags", tags);
    } catch (err) {
      console.error("Failed to load tags:", err);
    }
  },

  loadLocations: async () => {
    try {
      const locations = await getLocations();
      setMetadataState("locations", locations);
    } catch (err) {
      console.error("Failed to load locations:", err);
    }
  },

  loadStats: async () => {
    try {
      const stats = await tagService.getLibraryStats();
      const tagMap = new Map();
      stats.tag_counts.forEach(c => tagMap.set(c.tag_id, c.count));
      
      const folderMap = new Map();
      stats.folder_counts.forEach(c => folderMap.set(c.folder_id, c.count));
      
      const folderRecursiveMap = new Map();
      if (stats.folder_counts_recursive) {
        stats.folder_counts_recursive.forEach(c => folderRecursiveMap.set(c.folder_id, c.count));
      }

      setMetadataState("libraryStats", {
        total_images: stats.total_images,
        untagged_images: stats.untagged_images,
        tag_counts: tagMap,
        folder_counts: folderMap,
        folder_counts_recursive: folderRecursiveMap
      });
    } catch (err) {
      console.error("Failed to load library stats:", err);
    }
  },

  refreshAll: async () => {
    await Promise.all([
      metadataActions.loadTags(),
      metadataActions.loadLocations(),
      metadataActions.loadStats()
    ]);
  }
};

export { metadataState };
