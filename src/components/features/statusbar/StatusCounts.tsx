import { Component, Show } from "solid-js";
import { useLibrary, useSelection } from "../../../core/hooks";
import { X } from "lucide-solid";
import { Button } from "../../ui/Button";

export const StatusCounts: Component = () => {
    const lib = useLibrary();
    const selection = useSelection(); // Using existing hook

    const totalLoaded = () => lib.items.length;
    const totalFiltered = () => lib.totalItems;

    return (
        <div class="statusbar-section">
            <span title="Total items currently loaded in view">
                {totalLoaded()} Loaded
            </span>
             
             {/* Divider */}
            <span class="statusbar-divider" />

            <span title="Total items matching current filter (from backend)">
               {totalFiltered()} Total
            </span>

            <Show when={selection.selectedIds.length > 0}>
                <span class="statusbar-divider" />
                <span class="statusbar-selected">
                    {selection.selectedIds.length} Selected
                </span>
                <Button 
                    variant="ghost" 
                    size="icon-xs" 
                    class="status-btn" 
                    title="Clear selection (Esc)"
                    onClick={(e) => {
                        e.stopPropagation();
                        selection.clear();
                    }}
                >
                    <X size={12} />
                </Button>
            </Show>
        </div>
    );
};
