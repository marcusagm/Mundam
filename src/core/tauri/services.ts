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
  }
};
