import { Component, JSX } from "solid-js";
import "../styles/global.css";
import "./app-shell.css";

// This layout implements the 3-pane Grid structure
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

      {/* Main 3-Pane Area */}
      <div class="shell-body">
        {/* Left Sidebar */}
        <aside class="shell-sidebar">
          {props.sidebar}
        </aside>

        {/* Central Viewport */}
        <main class="shell-content">
          {props.children}
        </main>

        {/* Right Inspector */}
        <aside class="shell-inspector">
          {props.inspector}
        </aside>
      </div>

      {/* Footer / Statusbar */}
      <footer class="shell-footer">
        {props.statusbar}
      </footer>
    </div>
  );
};
