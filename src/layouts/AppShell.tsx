import { Component, JSX } from "solid-js";
import { ResizablePanelGroup, ResizablePanel, ResizableHandle } from "../components/ui/Resizable";
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
  return (
    <div class="app-shell">
      {/* Header Area */}
      <header class="shell-header">
        {props.header}
      </header>

      {/* Main 3-Pane Area - Resizable */}
      <ResizablePanelGroup direction="horizontal" class="shell-body">
        {/* Left Sidebar */}
        <ResizablePanel id="shell-sidebar" defaultSize={18} minSize={12} maxSize={35} class="shell-sidebar">
          {props.sidebar}
        </ResizablePanel>

        <ResizableHandle />

        {/* Central Viewport */}
        <ResizablePanel id="shell-content" defaultSize={62} minSize={30} class="shell-content">
          {props.children}
        </ResizablePanel>

        <ResizableHandle />

        {/* Right Inspector */}
        <ResizablePanel id="shell-inspector" defaultSize={20} minSize={15} maxSize={40} class="shell-inspector">
          {props.inspector}
        </ResizablePanel>
      </ResizablePanelGroup>

      {/* Footer / Statusbar */}
      <footer class="shell-footer">
        {props.statusbar}
      </footer>
    </div>
  );
};
