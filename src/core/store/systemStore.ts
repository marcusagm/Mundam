import { createSignal } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { addLocation, initDb } from "../../lib/db";
import { tauriService } from "../tauri/services";
import { metadataActions } from "./metadataStore";

export interface ProgressPayload {
  total: number;
  processed: number;
  current_file: string;
}

const [loading, setLoading] = createSignal(true);
const [progress, setProgress] = createSignal<ProgressPayload | null>(null);
const [rootPath, setRootPath] = createSignal<string | null>(null);
const [initialized, setInitialized] = createSignal(false);

export const systemActions = {
  initialize: async () => {
    if (initialized()) return;
    
    try {
      setLoading(true);
      await initDb();
      await metadataActions.loadLocations();
      await metadataActions.loadTags();
      
      // Auto-select root path if locations exist
      import("./metadataStore").then(({ metadataState }) => {
          if (metadataState.locations.length > 0) {
              const main = metadataState.locations[0];
              setRootPath(main.path);
              // Trigger initial load
               import("./libraryStore").then(({ libraryActions }) => {
                  libraryActions.refreshImages(true);
               });
               metadataActions.loadStats();
          }
      });
      
      // Setup Listeners
      listen<ProgressPayload>("indexer:progress", (e) => {
        systemActions.updateProgress(e.payload);
      });
      
      listen<number>("indexer:complete", () => {
        systemActions.clearProgress();
        // Refresh library to show new items
        // We use a small delay or just call it directly
        // Importing actions inside function to avoid circular dependency issues if any, 
        // though we are importing them at top level. 
        // Circular dependency libraryStore <-> systemStore might exist if not careful.
        // libraryStore imports systemStore? Previous check showed unused import removed.
        // libraryStore DOES NOT import systemStore anymore.
        // systemStore imports metadataStore.
        // We need libraryActions here.
        import("./libraryStore").then(({ libraryActions }) => {
            libraryActions.refreshImages(true);
        });
        metadataActions.loadStats();
        metadataActions.loadLocations();
      });

      listen<{id: number, path: string}>("thumbnail:ready", (e) => {
         import("./libraryStore").then(({ libraryActions }) => {
            libraryActions.updateThumbnail(e.payload.id, e.payload.path);
         });
      });

      setInitialized(true);
    } catch (err) {
      console.error("Initialization failed:", err);
    } finally {
      setLoading(false);
    }
  },

  setRootLocation: async (path: string) => {
    await addLocation(path);
    await metadataActions.loadLocations();
    setRootPath(path);
    await tauriService.startIndexing({ path });
  },

  updateProgress: (payload: ProgressPayload) => {
    setProgress(payload);
  },

  clearProgress: () => {
    setProgress(null);
  },

  setRootPath: (path: string | null) => {
    setRootPath(path);
  },
  
  setLoading: (isLoading: boolean) => {
    setLoading(isLoading);
  }
};

export { loading, progress, rootPath };
