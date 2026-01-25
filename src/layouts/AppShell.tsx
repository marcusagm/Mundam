import { Component, JSX } from "solid-js";
import "../styles/global.css";

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
    <div class="app-shell" style={{
        display: "grid",
        "grid-template-rows": "min-content 1fr min-content", /* Header, Main, Footer */
        "grid-template-columns": "1fr",
        height: "100vh",
        width: "100vw",
        overflow: "hidden"
    }}>
      {/* Header Area */}
      <header class="shell-header" style={{ "border-bottom": "1px solid var(--border-color)" }}>
        {props.header}
      </header>

      {/* Main 3-Pane Area */}
      <div class="shell-body" style={{
          display: "grid",
          "grid-template-columns": "260px 1fr 280px", /* Sidebar | Content | Inspector */
          "min-height": "0", /* Fix for nested scrolling */
          overflow: "hidden"
      }}>
        {/* Left Sidebar */}
        <aside class="shell-sidebar" style={{ 
            "border-right": "1px solid var(--border-color)",
            "background-color": "var(--surface-color)",
            "overflow-y": "auto"
        }}>
          {props.sidebar}
        </aside>

        {/* Central Viewport */}
        <main class="shell-content" style={{
            "position": "relative",
            "background-color": "var(--bg-color)",
            "overflow": "hidden" 
        }}>
          {props.children}
        </main>

        {/* Right Inspector */}
        <aside class="shell-inspector" style={{
            "border-left": "1px solid var(--border-color)",
            "background-color": "var(--surface-color)",
             "overflow-y": "auto"
        }}>
          {props.inspector}
        </aside>
      </div>

      {/* Footer / Statusbar */}
      <footer class="shell-footer" style={{ 
          "border-top": "1px solid var(--border-color)", 
          "padding": "0 8px",
          "height": "28px",
          "display": "flex",
          "align-items": "center",
          "font-size": "11px",
          "background-color": "var(--surface-color)"
      }}>
        {props.statusbar}
      </footer>
    </div>
  );
};
