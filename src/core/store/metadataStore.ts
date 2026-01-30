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

export interface SmartFolder {
  id: number;
  name: string;
  query_json: string;
  created_at: string;
}

interface MetadataState {
  tags: Tag[];
  locations: FolderNode[];
  smartFolders: SmartFolder[];
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
  smartFolders: [],
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
  loadSmartFolders: async () => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const folders = await invoke("get_smart_folders") as SmartFolder[];
      setMetadataState("smartFolders", folders);
    } catch (err) {
      console.error("Failed to load smart folders:", err);
    }
  },

  saveSmartFolder: async (name: string, query: any, id?: number) => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      if (id) {
          await invoke("update_smart_folder", { id, name, query: JSON.stringify(query) });
      } else {
          await invoke("save_smart_folder", { name, query: JSON.stringify(query) });
      }
      await metadataActions.loadSmartFolders();
    } catch (err) {
      console.error("Failed to save smart folder:", err);
    }
  },

  deleteSmartFolder: async (id: number) => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("delete_smart_folder", { id });
      await metadataActions.loadSmartFolders();
    } catch (err) {
      console.error("Failed to delete smart folder:", err);
    }
  },
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
      metadataActions.loadStats(),
      metadataActions.loadSmartFolders()
    ]);
  },

  handleBatchChange: (payload: any) => {
      let needsRefresh = false;
      // Get known location IDs to check for new folders
      const knownIds = new Set(metadataState.locations.map(l => l.id));

      if (payload.needs_refresh) {
          needsRefresh = true;
      }

      if (payload.added) {
          for (const item of payload.added) {
              // If item belongs to a folder we don't know about, we must refresh locations
              if (item.folder_id && !knownIds.has(item.folder_id)) {
                  needsRefresh = true;
                  break;
              }
          }
      }

      setMetadataState("libraryStats", (stats) => {
          const newStats = { ...stats };
          const tagCounts = new Map(stats.tag_counts);
          const folderCounts = new Map(stats.folder_counts);
          const folderRecursive = new Map(stats.folder_counts_recursive);
          
          let totalDiff = 0;
          let untaggedDiff = 0;

          const getAncestors = (folderId: number): number[] => {
              const ancestors: number[] = [];
              let currentId: number | null = folderId;
              while (currentId) {
                  ancestors.push(currentId);
                  const node = metadataState.locations.find(n => n.id === currentId);
                  currentId = node ? node.parent_id : null;
              }
              return ancestors;
          };

          // Removed
          if (payload.removed) {
              for (const item of payload.removed) {
                  totalDiff--;
                  if (!item.tag_ids || item.tag_ids.length === 0) {
                      untaggedDiff--;
                  } else {
                      for (const tagId of item.tag_ids) {
                          const c = tagCounts.get(tagId) || 0;
                          if (c > 0) tagCounts.set(tagId, c - 1);
                      }
                  }
                  
                  // Folder
                  if (item.folder_id) {
                      const fc = folderCounts.get(item.folder_id) || 0;
                      if (fc > 0) folderCounts.set(item.folder_id, fc - 1);
                      
                      const ancestors = getAncestors(item.folder_id);
                      for (const aid of ancestors) {
                          const rc = folderRecursive.get(aid) || 0;
                          if (rc > 0) folderRecursive.set(aid, rc - 1);
                      }
                  }
              }
          }

          // Added
           if (payload.added) {
              for (const item of payload.added) {
                  totalDiff++;
                  // Assumptions new items are untagged
                  untaggedDiff++; 
                  
                  if (item.folder_id) {
                      const fc = folderCounts.get(item.folder_id) || 0;
                      folderCounts.set(item.folder_id, fc + 1);
                      
                      const ancestors = getAncestors(item.folder_id);
                      for (const aid of ancestors) {
                          const rc = folderRecursive.get(aid) || 0;
                          folderRecursive.set(aid, rc + 1);
                      }
                  }
              }
           }

           // Updated (Moves)
           if (payload.updated) {
              for (const item of payload.updated) {
                  if (item.old_folder_id && item.old_folder_id !== item.folder_id) {
                      // Decrement Old
                      const oldFc = folderCounts.get(item.old_folder_id) || 0;
                      if (oldFc > 0) folderCounts.set(item.old_folder_id, oldFc - 1);
                      
                      const oldAncestors = getAncestors(item.old_folder_id);
                      for (const aid of oldAncestors) {
                          const rc = folderRecursive.get(aid) || 0;
                          if (rc > 0) folderRecursive.set(aid, rc - 1);
                      }

                      // Increment New
                      const newFc = folderCounts.get(item.folder_id) || 0;
                      folderCounts.set(item.folder_id, newFc + 1);
                      
                      const newAncestors = getAncestors(item.folder_id);
                      for (const aid of newAncestors) {
                          const rc = folderRecursive.get(aid) || 0;
                          folderRecursive.set(aid, rc + 1);
                      }
                  }
                  
                  // Check if new folder is unknown
                  if (item.folder_id && !knownIds.has(item.folder_id)) {
                      needsRefresh = true;
                  }
              }
           }

          newStats.total_images += totalDiff;
          newStats.untagged_images += untaggedDiff;
          newStats.tag_counts = tagCounts;
          newStats.folder_counts = folderCounts;
          newStats.folder_counts_recursive = folderRecursive;
          
          return newStats;
      });

      if (needsRefresh) {
          console.log("New folders detected, refreshing all metadata...");
          metadataActions.refreshAll();
      }
  }
};

export { metadataState };
