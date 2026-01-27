import { Component } from "solid-js";
import { Layers, Tag } from "lucide-solid";
import { useAppStore, appActions } from "../../../core/store/appStore";
import { CountBadge } from "../../ui/CountBadge";
import { SidebarPanel } from "../../ui/SidebarPanel";

export const LibrarySidebarPanel: Component = () => {
    const { state } = useAppStore();

    return (
        <SidebarPanel title="Library" class="panel-fixed">
            <div 
                class={`nav-item ${(!state.selectedLocationId && !state.filterUntagged && state.selectedTags.length === 0) ? 'active' : ''}`}
                onClick={() => appActions.clearAllFilters()}
            >
                <Layers size={16} />
                <span style={{ flex: 1 }}>All Items</span>
                <CountBadge count={state.libraryStats.total_images} variant="secondary" />
            </div>
            <div 
                class={`nav-item ${state.filterUntagged ? 'active' : ''}`}
                onClick={() => appActions.toggleUntagged()}
            >
                <Tag size={16} />
                <span style={{ flex: 1 }}>Untagged</span>
                <CountBadge count={state.libraryStats.untagged_images} variant="secondary" />
            </div>
        </SidebarPanel>
    );
};
