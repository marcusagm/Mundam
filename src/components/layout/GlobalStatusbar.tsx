import { Component } from "solid-js";
import { useAppStore } from "../../core/store/appStore";

export const GlobalStatusbar: Component = () => {
    const { state } = useAppStore();

  return (
    <div style={{ display: "flex", width: "100%", "justify-content": "space-between", "padding": "0 4px" }}>
        <div style={{ display: "flex", gap: "12px" }}>
            <span>{state.items.length} Items</span>
            <span>{state.selection.length} Selected</span>
        </div>
        <div>
            <span>Zoom: 100%</span>
        </div>
    </div>
  );
};
