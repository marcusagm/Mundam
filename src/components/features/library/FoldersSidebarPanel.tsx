import { Component, For } from "solid-js";
import { Folder, Plus } from "lucide-solid";
import { useAppStore, appActions } from "../../../core/store/appStore";
import { CountBadge } from "../../ui/CountBadge";
import { Button } from "../../ui/Button";
import { SidebarPanel } from "../../ui/SidebarPanel";

export const FoldersSidebarPanel: Component = () => {
    const { state } = useAppStore();

    return (
        <SidebarPanel 
            title="Folders" 
            class="panel-limited"
            actions={
                <Button variant="ghost" size="icon-sm" title="Create Folder">
                    <Plus size={14} />
                </Button>
            }
        >
            <div class="sidebar-list">
                <For each={state.locations}>
                    {(loc) => (
                        <div 
                            class={`nav-item ${state.selectedLocationId === (loc as any).id ? 'active' : ''}`} 
                            title={loc.path}
                            onClick={() => appActions.selectLocation((loc as any).id)}
                        >
                            <Folder size={16} fill="var(--text-muted)" stroke="none" /> 
                            <span class="truncate" style={{ flex: 1 }}>
                                {loc.name}
                            </span>
                            <CountBadge count={state.libraryStats.folder_counts.get((loc as any).id) || 0} variant="outline" />
                        </div>
                    )}
                </For>
                
                {state.locations.length === 0 && (
                    <div class="sidebar-empty-state">
                        No folders linked
                    </div>
                )}
            </div>
        </SidebarPanel>
    );
};
