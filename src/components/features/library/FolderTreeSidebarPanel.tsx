import { Component, createMemo, createSignal, onMount, createResource } from "solid-js";
import { Folder, FolderOpen, Plus } from "lucide-solid";
import { useMetadata, useFilters } from "../../../core/hooks";
import { TreeView, TreeNode } from "../../ui/TreeView";
import { SidebarPanel } from "../../ui/SidebarPanel";
import { Button } from "../../ui/Button";
import { CountBadge } from "../../ui/CountBadge";
import { FolderDeleteModal } from "./FolderDeleteModal";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { libraryActions } from "../../../core/store/libraryStore";
import "./folder-tree-sidebar-panel.css";

interface Location {
    id: number;
    path: string;
    name: string;
}

interface Subfolder {
    id: number;
    location_id: number;
    parent_id: number | null;
    relative_path: string;
    name: string;
}

interface FolderNodeData {
    type: 'location' | 'subfolder';
    locationId: number;
    subfolderId?: number;
    path?: string;
    name?: string;
}

export const FolderTreeSidebarPanel: Component = () => {
    const metadata = useMetadata();
    const filters = useFilters();
    
    const [expandedIds, setExpandedIds] = createSignal<Set<string | number>>(new Set());
    const [deleteModalOpen, setDeleteModalOpen] = createSignal(false);
    const [locationToDelete, setLocationToDelete] = createSignal<Location | null>(null);
    const [isDragOver] = createSignal(false);
    
    // Fetch subfolders
    const [subfolders, { refetch: refetchSubfolders }] = createResource(
        () => metadata.locations.length,
        async () => {
            const result = await invoke<Subfolder[]>("get_all_subfolders");
            return result;
        }
    );
    
    // Fetch subfolder counts
    const [subfolderCounts, { refetch: refetchCounts }] = createResource(
        () => metadata.locations.length,
        async () => {
            const results = await Promise.all([
                invoke<[number, number][]>("get_subfolder_counts"),
                invoke<[number, number][]>("get_location_root_counts")
            ]);
            return {
                subfolder: new Map(results[0]),
                root: new Map(results[1])
            };
        }
    );
    
    // Load/Save expansion state
    onMount(() => {
        const saved = localStorage.getItem("elleven_folder_expanded");
        if (saved) {
            try {
                const parsed = JSON.parse(saved);
                if (Array.isArray(parsed)) setExpandedIds(new Set(parsed));
            } catch (e) { console.error(e); }
        }
    });

    const toggleExpansion = (id: string | number) => {
        setExpandedIds(prev => {
            const next = new Set(prev);
            if (next.has(id)) next.delete(id);
            else next.add(id);
            localStorage.setItem("elleven_folder_expanded", JSON.stringify(Array.from(next)));
            return next;
        });
    };

    // Build tree structure
    const folderTree = createMemo(() => {
        const locations = metadata.locations || [];
        const subs = subfolders() || [];
        const counts = subfolderCounts() || { subfolder: new Map(), root: new Map() };
        const roots: TreeNode[] = [];
        
        // Create location nodes (root folders)
        for (const loc of locations) {
            const locationId = (loc as any).id;
            const locationSubfolders = subs.filter(s => s.location_id === locationId);
            
            // Build subfolder map
            const subfolderMap = new Map<number, TreeNode>();
            for (const sf of locationSubfolders) {
                subfolderMap.set(sf.id, {
                    id: `sf-${sf.id}`,
                    label: sf.name,
                    children: [],
                    data: { type: 'subfolder', subfolderId: sf.id, locationId },
                    icon: Folder,
                    badge: <CountBadge count={counts.subfolder?.get(sf.id) || 0} variant="secondary" />
                });
            }
            
            // Build hierarchy
            const directChildren: TreeNode[] = [];
            for (const sf of locationSubfolders) {
                const node = subfolderMap.get(sf.id)!;
                if (sf.parent_id && subfolderMap.has(sf.parent_id)) {
                    subfolderMap.get(sf.parent_id)!.children!.push(node);
                } else {
                    directChildren.push(node);
                }
            }
            
            // Count images in root folder (not in subfolders)
            // Use root counts from DB, fallback to 0
            const rootCount = counts.root?.get(locationId) || 0;
            
            const locationNode: TreeNode = {
                id: `loc-${locationId}`,
                label: loc.name,
                children: directChildren,
                data: { type: 'location', locationId, path: loc.path, name: loc.name },
                icon: FolderOpen,
                badge: <CountBadge count={rootCount} variant="secondary" />
            };
            
            roots.push(locationNode);
        }
        
        return roots;
    });

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
                metadata.refreshAll();
                refetchSubfolders();
                refetchCounts();
            }
        } catch (err) {
            console.error("Failed to add folder:", err);
        }
    };
    
    const handleSelect = (node: TreeNode) => {
        const data = node.data as FolderNodeData;
        console.log("FolderTree: handleSelect", { type: data.type, locationId: data.locationId, subfolderId: data.subfolderId });
        if (data.type === 'location') {
            // Use setFolder to set both atomically (subfolderId = -1 means "root only")
            filters.setFolder(data.locationId, -1);
        } else if (data.type === 'subfolder') {
            filters.setFolder(data.locationId, data.subfolderId!);
        }
    };
    
    const handleContextMenu = (e: MouseEvent, node: TreeNode) => {
        e.preventDefault();
        const data = node.data as FolderNodeData;
        if (data.type === 'location') {
            setLocationToDelete({
                id: data.locationId,
                path: data.path!,
                name: data.name!
            });
            setDeleteModalOpen(true);
        }
    };
    
    const handleDeleted = () => {
        metadata.refreshAll();
        refetchSubfolders();
        refetchCounts();
        if (filters.selectedLocationId === locationToDelete()?.id) {
            filters.setLocation(null);
            filters.setSubfolder(null);
        }
        libraryActions.refreshImages(true);
    };
    
    // Get selected IDs for tree
    const selectedIds = createMemo(() => {
        const ids: (string | number)[] = [];
        if (filters.selectedLocationId) {
            // If we have a specfic subfolder > 0
            if (filters.selectedSubfolderId && filters.selectedSubfolderId > 0) {
                ids.push(`sf-${filters.selectedSubfolderId}`);
            } else {
                // Otherwise it's the root location selected (either -1 or null)
                ids.push(`loc-${filters.selectedLocationId}`);
            }
        }
        return ids;
    });

    return (
        <>
            <SidebarPanel 
                title="Folders" 
                class={`panel-fluid ${isDragOver() ? "drag-over" : ""}`}
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
                {folderTree().length > 0 ? (
                    <TreeView 
                        items={folderTree()} 
                        onSelect={handleSelect}
                        selectedIds={selectedIds()} 
                        onContextMenu={handleContextMenu}
                        expandedIds={expandedIds()}
                        onToggle={toggleExpansion}
                    />
                ) : (
                    <div class="sidebar-empty-state">
                        <p>No folders linked</p>
                        <p class="empty-hint">Click + to add a folder</p>
                    </div>
                )}
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
