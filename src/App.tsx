import { onMount, Show } from "solid-js";
import { useSystem } from "./core/hooks";
import { AppShell } from "./layouts/AppShell";
import { PrimaryHeader } from "./components/layout/PrimaryHeader";
import { LibrarySidebar } from "./components/layout/LibrarySidebar";
import { FileInspector } from "./components/layout/FileInspector";
import { GlobalStatusbar } from "./components/layout/GlobalStatusbar";
import { Viewport } from "./components/layout/Viewport";
import { open } from "@tauri-apps/plugin-dialog";
// Native DnD
import { dndRegistry, TagDropStrategy, ImageDropStrategy } from "./core/dnd";
import { Loader } from "./components/ui/Loader";

import { useKeyboardShortcuts } from "./core/hooks/useKeyboardShortcuts";

function App() {
  const system = useSystem();
  
  useKeyboardShortcuts();

  onMount(() => {
    system.initialize();
    
    // Register Strategies
    dndRegistry.register("TAG", TagDropStrategy);
    dndRegistry.register("IMAGE", ImageDropStrategy);
    
    
    // Check if shift key held during start?
    
    // Notify Splah Screen
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
      </Show>
    </Show>
  );
}

export default App;
