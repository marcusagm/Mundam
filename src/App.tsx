import { onMount, Show, createEffect } from "solid-js";
import { useSystem, useNotification } from "./core/hooks";
import { AppShell } from "./layouts/AppShell";
import { PrimaryHeader } from "./components/layout/PrimaryHeader";
import { LibrarySidebar } from "./components/layout/LibrarySidebar";
import { FileInspector } from "./components/layout/FileInspector";
import { GlobalStatusbar } from "./components/layout/GlobalStatusbar";
import { Viewport } from "./components/layout/Viewport";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
// Native DnD
import { dndRegistry, TagDropStrategy, ImageDropStrategy, currentDragItem, setDropTargetId } from "./core/dnd";
import { Sonner } from "./components/ui/Sonner";
import { Loader } from "./components/ui/Loader";
import { useKeyboardShortcuts } from "./core/hooks/useKeyboardShortcuts";

function App() {
  const system = useSystem();
  const notification = useNotification();
  
  useKeyboardShortcuts();

  // Root-level DND cleanup
  createEffect(() => {
    if (!currentDragItem()) {
      setDropTargetId(null);
    }
  });

  onMount(() => {
    system.initialize();
    
    // Register Strategies
    dndRegistry.register("TAG", TagDropStrategy);
    dndRegistry.register("IMAGE", ImageDropStrategy);
    
    // Listen for indexing completion
    listen("indexer:complete", () => {
      notification.success("Indexing Complete", "Library update finished");
    });
    
    // Notify Splash Screen
    window.dispatchEvent(new CustomEvent('app-ready'));
  });

  const handleSelectFolder = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select Reference Library Folder",
      });
      
      if (selected) {
        const path = typeof selected === 'string' ? selected : (selected as any).path;
        if (path) {
          notification.info("Indexing Started", `Processing folder: ${path.split(/[\\/]/).pop()}`);
          await system.setRootLocation(path);
        }
      }
    } catch (err) {
      console.error("Failed to select folder:", err);
    }
  };

  return (
    <Show when={!system.loading()} fallback={<Loader fullscreen text="Initializing Elleven Library..." />}>
      <Show 
        when={system.rootPath()} 
        fallback={
          <div class="welcome-screen">
            <h1>Elleven Library</h1>
            <p>Start by choosing a folder to monitor for visual references.</p>
            <button class="primary-btn" onClick={handleSelectFolder}>
              Initialize Library
            </button>
          </div>
        }
      >
        <AppShell
            header={<PrimaryHeader />}
            sidebar={<LibrarySidebar />}
            inspector={<FileInspector />}
            statusbar={<GlobalStatusbar />}
        >
            <Viewport />
        </AppShell>
        <Sonner position="bottom-right" richColors />
      </Show>
    </Show>
  );
}

export default App;
