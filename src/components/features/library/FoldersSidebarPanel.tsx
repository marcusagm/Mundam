import { Component, For, createSignal, Show } from "solid-js";
import { Folder, Plus, Trash2 } from "lucide-solid";
import { useMetadata, useFilters } from "../../../core/hooks";
import { CountBadge } from "../../ui/CountBadge";
import { Button } from "../../ui/Button";
import { SidebarPanel } from "../../ui/SidebarPanel";
import { FolderDeleteModal } from "./FolderDeleteModal";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { libraryActions } from "../../../core/store/libraryStore";
import "./folders-sidebar-panel.css";

interface Location {
    id: number;
    path: string;
    name: string;
}

export const FoldersSidebarPanel: Component = () => {
    const metadata = useMetadata();
    const filters = useFilters();
    
    const [hoveredId, setHoveredId] = createSignal<number | null>(null);
    const [deleteModalOpen, setDeleteModalOpen] = createSignal(false);
    const [locationToDelete, setLocationToDelete] = createSignal<Location | null>(null);
    const [isDragOver, setIsDragOver] = createSignal(false);

    const handleAddFolder = async () => {
        try {
            const selected = await open({
                directory: true,
                multiple: false,
                title: "Select folder to add to library"
            });
            
            if (selected) {
                console.log("Adding folder:", selected);
                await invoke("add_location", { path: selected });
                // Refresh metadata to show new location
                metadata.refreshAll();
            }
        } catch (err) {
            console.error("Failed to add folder:", err);
        }
    };
    
    const handleDeleteClick = (e: MouseEvent, loc: any) => {
        e.stopPropagation();
        setLocationToDelete({
            id: loc.id,
            path: loc.path,
            name: loc.name
        });
        setDeleteModalOpen(true);
    };
    
    const handleDeleted = () => {
        metadata.refreshAll();
        // If we deleted the currently selected location, clear filter
        if (filters.selectedLocationId === locationToDelete()?.id) {
            filters.setLocation(null);
        }
        libraryActions.refreshImages(true);
    };
    
    // Drag and drop handlers
    const handleDragOver = (e: DragEvent) => {
        e.preventDefault();
        setIsDragOver(true);
    };
    
    const handleDragLeave = () => {
        setIsDragOver(false);
    };
    
    const handleDrop = async (e: DragEvent) => {
        e.preventDefault();
        setIsDragOver(false);
        
        const items = e.dataTransfer?.items;
        if (!items) return;
        
        for (let i = 0; i < items.length; i++) {
            const item = items[i];
            if (item.kind === "file") {
                const entry = item.webkitGetAsEntry?.();
                if (entry && entry.isDirectory) {
                    // Unfortunately we can't get the full path from webkitGetAsEntry in a sandboxed environment
                    // The user needs to use the dialog instead
                    console.log("Drag & drop detected directory, but path access is sandboxed");
                    // Fallback: open dialog
                    handleAddFolder();
                    break;
                }
            }
        }
    };

    return (
        <>
            <SidebarPanel 
                title="Folders" 
                class={`panel-limited ${isDragOver() ? "drag-over" : ""}`}
                actions={
                    <Button 
                        variant="ghost" 
                        size="icon-xs" 
                        title="Add Folder"
                        onClick={handleAddFolder}
                    >
                        <Plus size={14} />
                    </Button>
                }
            >
                <div 
                    class="sidebar-list"
                    onDragOver={handleDragOver}
                    onDragLeave={handleDragLeave}
                    onDrop={handleDrop}
                >
                    <For each={metadata.locations}>
                        {(loc) => (
                            <div 
                                class={`nav-item ${filters.selectedLocationId === (loc as any).id ? 'active' : ''}`} 
                                title={loc.path}
                                onClick={() => filters.setLocation((loc as any).id)}
                                onMouseEnter={() => setHoveredId((loc as any).id)}
                                onMouseLeave={() => setHoveredId(null)}
                            >
                                <Folder size={16} fill="var(--text-muted)" stroke="none" /> 
                                <span class="truncate" style={{ flex: 1 }}>
                                    {loc.name}
                                </span>
                                
                                <Show when={hoveredId() === (loc as any).id}>
                                    <button 
                                        class="folder-delete-btn"
                                        onClick={(e) => handleDeleteClick(e, loc)}
                                        title="Remove folder"
                                    >
                                        <Trash2 size={14} />
                                    </button>
                                </Show>
                                
                                <Show when={hoveredId() !== (loc as any).id}>
                                    <CountBadge 
                                        count={metadata.stats.folder_counts.get((loc as any).id) || 0} 
                                        variant="secondary" 
                                    />
                                </Show>
                            </div>
                        )}
                    </For>
                    
                    {metadata.locations.length === 0 && (
                        <div 
                            class={`sidebar-empty-state ${isDragOver() ? "drag-over" : ""}`}
                        >
                            <p>No folders linked</p>
                            <p class="empty-hint">Click + or drag a folder here</p>
                        </div>
                    )}
                </div>
            </SidebarPanel>
            
            <FolderDeleteModal 
                isOpen={deleteModalOpen()}
                onClose={() => setDeleteModalOpen(false)}
                location={locationToDelete()}
                onDeleted={handleDeleted}
            />
        </>
    );
};
