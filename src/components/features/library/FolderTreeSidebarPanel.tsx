import { Component, createMemo, createSignal, onMount } from "solid-js";
import { Folder, FolderOpen, Plus } from "lucide-solid";
import { useMetadata, useFilters } from "../../../core/hooks";
import { TreeView, TreeNode } from "../../ui/TreeView";
import { SidebarPanel } from "../../ui/SidebarPanel";
import { Button } from "../../ui/Button";
import { CountBadge } from "../../ui/CountBadge";
import { FolderDeleteModal } from "./FolderDeleteModal";
import { FolderContextMenu } from "./FolderContextMenu";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { libraryActions } from "../../../core/store/libraryStore";
import "./folder-tree-sidebar-panel.css";

interface FolderNodeData {
    folderId: number;
    path: string;
    name: string;
    isRoot: boolean;
}

export const FolderTreeSidebarPanel: Component = () => {
    const metadata = useMetadata();
    const filters = useFilters();
    
    const [expandedIds, setExpandedIds] = createSignal<Set<string | number>>(new Set());
    const [deleteModalOpen, setDeleteModalOpen] = createSignal(false);
    const [folderToDelete, setFolderToDelete] = createSignal<FolderNodeData | null>(null);
    const [contextMenuOpen, setContextMenuOpen] = createSignal(false);
    const [contextMenuPos, setContextMenuPos] = createSignal({ x: 0, y: 0 });
    const [contextMenuNode, setContextMenuNode] = createSignal<TreeNode | null>(null);
    const [isDragOver] = createSignal(false);
    
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
        const allFolders = metadata.locations || [];
        const isRecursive = filters.folderRecursiveView;
        const counts = isRecursive ? metadata.stats.folder_counts_recursive : metadata.stats.folder_counts;
        
        // Map ID -> TreeNode
        const nodeMap = new Map<number, TreeNode>();
        const roots: TreeNode[] = [];

        // 1. Create all nodes
        for (const f of allFolders) {
            nodeMap.set(f.id, {
                id: `folder-${f.id}`,
                label: f.name,
                children: [],
                data: { 
                    folderId: f.id, 
                    path: f.path, 
                    name: f.name,
                    isRoot: f.is_root 
                } as FolderNodeData,
                icon: f.is_root ? FolderOpen : Folder,
                badge: <CountBadge showZero={true} count={counts.get(f.id) || 0} variant="secondary" />
            });
        }

        // 2. Build Hierarchy
        for (const f of allFolders) {
            const node = nodeMap.get(f.id)!;
            if (f.parent_id && nodeMap.has(f.parent_id)) {
                nodeMap.get(f.parent_id)!.children!.push(node);
            } else {
                // No parent (or parent not found/loaded) -> treat as root for display
                roots.push(node);
            }
        }
        
        // Sort roots?
        roots.sort((a, b) => a.label.localeCompare(b.label));

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
                await metadata.loadLocations();
                await metadata.loadStats();
            }
        } catch (err) {
            console.error("Failed to add folder:", err);
        }
    };
    
    const handleSelect = (node: TreeNode) => {
        const data = node.data as FolderNodeData;
        filters.setFolder(data.folderId);
    };

    const handleContextMenu = (e: MouseEvent, node: TreeNode) => {
        e.preventDefault();
        setContextMenuNode(node);
        setContextMenuPos({ x: e.clientX, y: e.clientY });
        setContextMenuOpen(true);
    };

    const handleDeleted = async () => {
        await metadata.loadLocations();
        await metadata.loadStats();
        if (filters.selectedFolderId === folderToDelete()?.folderId) {
            filters.setFolder(null);
        }
        libraryActions.refreshImages(true);
    };
    
    // Get selected IDs for tree
    const selectedIds = createMemo(() => {
        const ids: (string | number)[] = [];
        if (filters.selectedFolderId) {
            ids.push(`folder-${filters.selectedFolderId}`);
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
                location={folderToDelete() ? {
                    id: folderToDelete()!.folderId,
                    name: folderToDelete()!.name,
                    path: folderToDelete()!.path
                } : null}
                onDeleted={handleDeleted}
            />
            
            <FolderContextMenu 
                isOpen={contextMenuOpen()}
                x={contextMenuPos().x}
                y={contextMenuPos().y}
                node={contextMenuNode()}
                onClose={() => setContextMenuOpen(false)}
                onDelete={(node) => {
                    setFolderToDelete(node.data as FolderNodeData);
                    setDeleteModalOpen(true);
                }}
            />
        </>
    );
};
