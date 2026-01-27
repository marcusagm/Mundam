import { Component, Show } from "solid-js";
import { useAppStore } from "../../core/store/appStore";
import "./global-statusbar.css";

export const GlobalStatusbar: Component = () => {
    const { state } = useAppStore();

  return (
    <div class="global-statusbar">
        <div class="statusbar-section">
            <span>{state.items.length} Items</span>
            <Show when={state.selection.length > 0}>
                <span class="statusbar-selected">{state.selection.length} Selected</span>
            </Show>
        </div>
        <div class="statusbar-section">
             {/* Future: Sync Status, Cloud Icon */}
            <span>100%</span>
        </div>
    </div>
  );
};
