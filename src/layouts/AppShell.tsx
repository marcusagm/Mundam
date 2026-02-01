import { Component, JSX, createSignal, Show, createContext, useContext, Accessor } from "solid-js";
import { ResizablePanelGroup, ResizablePanel, ResizableHandle } from "../components/ui/Resizable";

// Context for Statusbar to control Shell
interface AppShellContextValue {
    isSidebarOpen: Accessor<boolean>;
    toggleSidebar: () => void;
    isInspectorOpen: Accessor<boolean>;
    toggleInspector: () => void;
}
const AppShellContext = createContext<AppShellContextValue>();
export const useAppShell = () => useContext(AppShellContext);
import "../styles/global.css";
import "./app-shell.css";

// This layout implements the 3-pane Grid structure with resizable areas
// [ Header ]
// [ Sidebar | Content | Inspector ]
// [ Statusbar ]

interface AppShellProps {
  children: JSX.Element;
  header?: JSX.Element;
  sidebar?: JSX.Element;
  inspector?: JSX.Element;
  statusbar?: JSX.Element;
}

export const AppShell: Component<AppShellProps> = (props) => {
  // Persistence Keys
  const STORAGE_KEY_LAYOUT = "app-shell-layout";
  const STORAGE_KEY_STATES = "app-shell-states";

  // Get persisted sizes or use defaults
  const getPersistedLayout = () => {
    try {
      const saved = localStorage.getItem(STORAGE_KEY_LAYOUT);
      return saved ? JSON.parse(saved) : null;
    } catch {
      return null;
    }
  };

  const getPersistedStates = () => {
      try {
          const saved = localStorage.getItem(STORAGE_KEY_STATES);
          return saved ? JSON.parse(saved) : { sidebar: true, inspector: true };
      } catch {
          return { sidebar: true, inspector: true };
      }
  };

  const layout = getPersistedLayout();
  const states = getPersistedStates();

  const [isSidebarOpen, setIsSidebarOpen] = createSignal(states.sidebar);
  const [isInspectorOpen, setIsInspectorOpen] = createSignal(states.inspector);

  const saveStates = (sidebar: boolean, inspector: boolean) => {
      localStorage.setItem(STORAGE_KEY_STATES, JSON.stringify({ sidebar, inspector }));
  };

  const toggleSidebar = () => {
      setIsSidebarOpen(prev => {
          const next = !prev;
          saveStates(next, isInspectorOpen());
          return next;
      });
  };

  const toggleInspector = () => {
      setIsInspectorOpen(prev => {
          const next = !prev;
          saveStates(isSidebarOpen(), next);
          return next;
      });
  };

  const sidebarSize = layout?.[0] ?? 18;
  const contentSize = layout?.[1] ?? 62;
  const inspectorSize = layout?.[2] ?? 20;

  const handleLayoutChange = (sizes: number[]) => {
    localStorage.setItem(STORAGE_KEY_LAYOUT, JSON.stringify(sizes));
  };

  return (
    <div class="app-shell">
      {/* Header Area Removed */}


      {/* Main 3-Pane Area - Resizable */}
      <ResizablePanelGroup 
        direction="horizontal" 
        class="shell-body"
        onLayout={handleLayoutChange}
      >
        {/* Left Sidebar */}
        <Show when={isSidebarOpen()}>
            <ResizablePanel 
              id="shell-sidebar" 
              defaultSize={sidebarSize} 
              minSize={12} 
              maxSize={35} 
              class="shell-sidebar"
            >
              {props.sidebar}
            </ResizablePanel>
            <ResizableHandle />
        </Show>

        {/* Central Viewport */}
        <ResizablePanel 
          id="shell-content" 
          defaultSize={contentSize} 
          minSize={30} 
          flexGrow={1}
          class="shell-content"
        >
          {props.children}
        </ResizablePanel>

        <Show when={isInspectorOpen()}>
            <ResizableHandle />
            {/* Right Inspector */}
            <ResizablePanel 
              id="shell-inspector" 
              defaultSize={inspectorSize} 
              minSize={15} 
              maxSize={40} 
              class="shell-inspector"
            >
              {props.inspector}
            </ResizablePanel>
        </Show>
      </ResizablePanelGroup>

      {/* Footer / Statusbar */}
      <footer class="shell-footer">
        <AppShellContext.Provider value={{ 
              isSidebarOpen, toggleSidebar, 
              isInspectorOpen, toggleInspector 
          }}>
            {props.statusbar}
        </AppShellContext.Provider>
      </footer>
    </div>
  );
};
