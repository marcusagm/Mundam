import { Component, Show } from "solid-js";
import { appActions, useAppStore } from "../../core/store/appStore";

export const PrimaryHeader: Component = () => {
  const { searchQuery, progress } = useAppStore();

  return (
    <div style={{ padding: "0 12px", height: "48px", display: "flex", "align-items": "center", gap: "12px" }}>
       {/* Navigation Buttons Placeholder */}
       <div style={{ display: "flex", gap: "4px" }}>
         <button disabled style={{ opacity: 0.5 }}>&lt;</button>
         <button disabled style={{ opacity: 0.5 }}>&gt;</button>
       </div>

       {/* OmniSearch */}
       <div style={{ flex: 1, display: "flex", "justify-content": "center" }}>
           <input 
             type="text" 
             placeholder="Search references (Ctrl+K)" 
             value={searchQuery()}
             onInput={(e) => appActions.setSearch(e.currentTarget.value)}
             style={{ 
                 width: "100%", 
                 "max-width": "500px",
                 "background-color": "var(--bg-color)",
                 "border": "1px solid var(--border-color)",
                 "border-radius": "6px",
                 "padding": "6px 12px",
                 "font-size": "13px"
             }} 
            />
       </div>

       {/* Actions / Status */}
       <div style={{ display: "flex", "align-items": "center", gap: "12px", "font-size": "12px" }}>
            <Show when={progress()}>
                <div style={{ display: "flex", "align-items": "center", gap: "6px", color: "var(--accent-color)" }}>
                    <div class="status-dot"></div>
                    <span>Indexing {progress()?.processed} / {progress()?.total}</span>
                </div>
            </Show>
            <button class="primary-btn" style={{ "font-size": "11px", padding: "4px 12px" }}>+ Add</button>
       </div>
    </div>
  );
};
