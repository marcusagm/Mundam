import { Component, Show, createMemo } from "solid-js";
import { useAppStore } from "../../core/store/appStore";

export const FileInspector: Component = () => {
  const { state } = useAppStore();

  // Derived state for the active item
  const activeItem = createMemo(() => {
    if (state.selection.length === 0) return null;
    // Show the last selected item (or primary selection)
    const id = state.selection[state.selection.length - 1];
    return state.items.find((i) => i.id === id);
  });

  return (
    <div style={{ height: "100%" }}>
        <Show when={activeItem()} fallback={
            <div style={{ display: "flex", "align-items": "center", "justify-content": "center", height: "100%", color: "var(--text-muted)", "font-size": "12px" }}>
                Select an item to view details
            </div>
        }>
            <div style={{ padding: "12px" }}>
                <div style={{ "margin-bottom": "16px" }}>
                    <div class="reference-image-container" style={{ "aspect-ratio": "1/1", "border-radius": "4px", "background": "#000" }}>
                       {/* Simplified preview here, or reuse ReferenceImage without interaction */}
                       <img src={activeItem()?.thumbnail_path ? `thumb://localhost/${activeItem()?.thumbnail_path?.split(/[\\/]/).pop()}` : ""} style={{ "object-fit": "contain" }} />
                    </div>
                </div>

                <div class="field-group">
                    <label style={{ display: "block", "font-size": "10px", color: "var(--text-secondary)", "margin-bottom": "4px" }}>Name</label>
                    <input type="text" value={activeItem()?.filename} disabled style={{ width: "100%" }} />
                </div>

                <div class="field-group" style={{ "margin-top": "12px" }}>
                    <label style={{ display: "block", "font-size": "10px", color: "var(--text-secondary)", "margin-bottom": "4px" }}>Tags</label>
                    <input type="text" placeholder="Add tags..." style={{ width: "100%" }} />
                </div>

                <div style={{ "margin-top": "24px", "border-top": "1px solid var(--border-color)", "padding-top": "12px" }}>
                    <div style={{ display: "flex", "justify-content": "space-between", "font-size": "11px", "margin-bottom": "4px" }}>
                        <span style={{ color: "var(--text-secondary)" }}>Dimensions</span>
                        <span>{activeItem()?.width} x {activeItem()?.height}</span>
                    </div>
                    <div style={{ display: "flex", "justify-content": "space-between", "font-size": "11px" }}>
                         <span style={{ color: "var(--text-secondary)" }}>ID</span>
                         <span>{activeItem()?.id}</span>
                    </div>
                </div>
            </div>
        </Show>
    </div>
  );
};
