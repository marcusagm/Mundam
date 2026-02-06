import { invoke } from "@tauri-apps/api/core";

// Define strict types for Tauri commands
export interface StartIndexingArgs {
  path: string;
}

export const tauriService = {
  /**
   * Starts the background indexing process for the given directory path.
   * This triggers the 'indexer:progress' and 'indexer:complete' events.
   */
  startIndexing: async (args: StartIndexingArgs): Promise<void> => {
    try {
      await invoke("start_indexing", { path: args.path });
    } catch (error) {
      console.error("Tauri command 'start_indexing' failed:", error);
      throw error;
    }
  },

  /**
   * Example wrapper for other commands...
   */
  // stopIndexing: async () => invoke("stop_indexing"),

  getLibrarySupportedFormats: async (): Promise<any[]> => {
      try {
          return await invoke("get_library_supported_formats");
      } catch (error) {
          console.error("Failed to load formats:", error);
          return [];
      }
  },

  runDbMaintenance: async (): Promise<void> => {
      try {
          await invoke("run_db_maintenance");
      } catch (error) {
          console.error("Failed to run DB maintenance:", error);
          throw error;
      }
  },

  getSetting: async (key: string): Promise<any> => {
      try {
          return await invoke("get_setting", { key });
      } catch (error) {
          console.error(`Failed to get setting ${key}:`, error);
          return null;
      }
  },

  setSetting: async (key: string, value: any): Promise<void> => {
      try {
          await invoke("set_setting", { key, value });
      } catch (error) {
          console.error(`Failed to set setting ${key}:`, error);
          throw error;
      }
  },

  // --- Cache Management ---

  getCacheStats: async (): Promise<{ directory: string; size_bytes: number; file_count: number }> => {
      try {
          return await invoke("get_cache_stats");
      } catch (error) {
          console.error("Failed to get cache stats:", error);
          return { directory: "", size_bytes: 0, file_count: 0 };
      }
  },

  cleanupCache: async (maxAgeDays?: number): Promise<number> => {
      try {
          return await invoke("cleanup_cache", { maxAgeDays });
      } catch (error) {
          console.error("Failed to cleanup cache:", error);
          throw error;
      }
  },

  clearCache: async (): Promise<number> => {
      try {
          return await invoke("clear_cache");
      } catch (error) {
          console.error("Failed to clear cache:", error);
          throw error;
      }
  }
};
