import { onMount, Show } from "solid-js";
import { appActions, useAppStore } from "./core/store/appStore";
import { AppShell } from "./layouts/AppShell";
import { PrimaryHeader } from "./components/layout/PrimaryHeader";
import { LibrarySidebar } from "./components/layout/LibrarySidebar";
import { FileInspector } from "./components/layout/FileInspector";
import { GlobalStatusbar } from "./components/layout/GlobalStatusbar";
import { VirtualMasonry } from "./components/features/viewport/VirtualMasonry";
import { open } from "@tauri-apps/plugin-dialog";

import { useKeyboardShortcuts } from "./core/hooks/useKeyboardShortcuts";

function App() {
  const { state, loading, rootPath, progress } = useAppStore();
  
  useKeyboardShortcuts();

  onMount(() => {
    appActions.initialize();
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
          const name = path.split(/[\/\\]/).pop() || path;
          await appActions.setRootLocation(path, name);
        }
      }
    } catch (err) {
      console.error("Failed to select folder:", err);
    }
  };

  return (
    <Show when={!loading()} fallback={<div class="grid-placeholder">Loading...</div>}>
      <Show 
        when={rootPath()} 
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
             <VirtualMasonry items={state.items} />
        </AppShell>
      </Show>
    </Show>
  );
}

export default App;
