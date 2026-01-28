import { Component } from "solid-js";
import { Layers, Tag } from "lucide-solid";
import { useMetadata, useFilters } from "../../../core/hooks";
import { CountBadge } from "../../ui/CountBadge";
import { SidebarPanel } from "../../ui/SidebarPanel";

export const LibrarySidebarPanel: Component = () => {
    const metadata = useMetadata();
    const filters = useFilters();

    return (
        <SidebarPanel title="Library" class="panel-fixed">
            <div 
                class={`nav-item ${(!filters.selectedFolderId && !filters.filterUntagged && filters.selectedTags.length === 0) ? 'active' : ''}`}
                onClick={() => filters.clearAll()}
            >
                <Layers size={16} />
                <span style={{ flex: 1 }}>All Items</span>
                <CountBadge count={metadata.stats.total_images} variant="secondary" />
            </div>
            <div 
                class={`nav-item ${filters.filterUntagged ? 'active' : ''}`}
                onClick={() => filters.toggleUntagged()}
            >
                <Tag size={16} />
                <span style={{ flex: 1 }}>Untagged</span>
                <CountBadge count={metadata.stats.untagged_images} variant="secondary" />
            </div>
        </SidebarPanel>
    );
};
