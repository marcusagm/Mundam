import { Component, For } from "solid-js";
import { useAppStore } from "../../core/store/appStore";

export const LibrarySidebar: Component = () => {
  const { state } = useAppStore();

  return (
    <div style={{ padding: "0" }}>
        <div style={{ padding: "12px", "font-size": "11px", "font-weight": "bold", color: "var(--text-muted)", "text-transform": "uppercase" }}>
            Library
        </div>
        
        {/* Smart Filters Placeholder */}
        <div class="nav-item active">All Items <span style={{ float: "right", opacity: 0.5 }}>{state.items.length}</span></div>
        <div class="nav-item">Uncategorized</div>
        <div class="nav-item">Trash</div>

        <div style={{ padding: "12px 12px 4px 12px", "font-size": "11px", "font-weight": "bold", color: "var(--text-muted)", "text-transform": "uppercase", "margin-top": "12px" }}>
            Folders
        </div>
        
        {/* Folder List from Store */}
        <For each={state.locations}>
            {(loc) => (
                <div class="nav-item" title={loc.path}>
                    📂 {loc.name}
                </div>
            )}
        </For>
    </div>
  );
};
