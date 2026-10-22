import { Button } from '../../ui';
import { Component, createMemo, createSignal } from 'solid-js';
// import { Folder as FolderIcon, FolderOpen as FolderOpenIcon, Plus } from 'lucide-solid';
import { Folder as FolderIcon, Plus } from 'lucide-solid';
import { useMetadata, useFilters, useNotification, useLibrary, useTree } from '../../../core/hooks';
import { TreeView, TreeNode } from '../../ui/TreeView';
import { SidebarPanel } from '../../ui/SidebarPanel';
import { CountBadge } from '../../ui/CountBadge';
import { FolderDeleteModal } from './FolderDeleteModal';
import { FolderContextMenu } from './FolderContextMenu';
import './folder-tree-sidebar-panel.css';

/**
 * Metadata for folder nodes to facilitate domain operations.
 */
interface FolderNodeData {
    /** The unique identifier of the folder. */
    folderIdentifier: string;
    /** The full file system path. */
    path: string;
    /** The display name of the folder. */
    name: string;
    /** Whether this is a root-level monitored folder. */
    isRoot: boolean;
}

/**
 * Sidebar panel for browsing the file system hierarchy linked to the library.
 */
export const FolderTreeSidebarPanel: Component = () => {
    const metadata = useMetadata();
    const filters = useFilters();
    const notification = useNotification();
    const tree = useTree('folders');

    // --- Component State ---
    const [deleteModalOpen, setDeleteModalOpen] = createSignal(false);
    const [folderToDelete, setFolderToDelete] = createSignal<FolderNodeData | null>(null);
    const [contextMenuOpen, setContextMenuOpen] = createSignal(false);
    const [contextMenuPosition, setContextMenuPosition] = createSignal({
        coordinateX: 0,
        coordinateY: 0
    });
    const [contextMenuNode, setContextMenuNode] = createSignal<TreeNode | null>(null);

    // --- Tree Construction ---
    const folderTreeHierarchy = createMemo(() => {
        const allFolders = metadata.locations || [];
        const isRecursiveMode = filters.folderRecursiveView;
        const counts = isRecursiveMode
            ? metadata.stats.folder_counts_recursive
            : metadata.stats.folder_counts;

        const nodeMap = new Map<string, TreeNode>();
        const rootNodes: TreeNode[] = [];

        // Phase 1: Create all nodes
        for (const folder of allFolders) {
            nodeMap.set(folder.id, {
                id: `folder-${folder.id}`,
                label: folder.name,
                children: [],
                data: {
                    folderIdentifier: folder.id,
                    path: folder.path,
                    name: folder.name,
                    isRoot: folder.is_root
                } as FolderNodeData,
                // icon: folder.is_root ? FolderOpenIcon : FolderIcon,
                icon: FolderIcon,
                badge: (
                    <CountBadge
                        showZero={true}
                        count={counts.get(folder.id) || 0}
                        variant="secondary"
                    />
                )
            });
        }

        // Phase 2: Build hierarchy
        for (const folder of allFolders) {
            const node = nodeMap.get(folder.id);
            if (!node) {
                continue;
            }

            if (folder.parent_id && nodeMap.has(folder.parent_id)) {
                nodeMap.get(folder.parent_id)!.children!.push(node);
            } else {
                rootNodes.push(node);
            }
        }

        return rootNodes.sort((a, b) => a.label.localeCompare(b.label));
    });

    /**
     * Opens a directory picker to add a new monitored folder to the library.
     */
    const handleAddFolder = () => {
        const { addLocation } = useLibrary();

        addLocation().then(result => {
            if (result.success && result.path) {
                notification.success(
                    'Folder Linked',
                    `Monitoring "${result.path.split(/[\\/]/).pop()}"`
                );
            } else if (result.error) {
                notification.error('Failed to Link Folder');
            }
        });
    };

    /**
     * Handles the selection of a folder node.
     *
     * @param node - The selected tree node.
     */
    const handleSelect = (node: TreeNode, isAdditive = false) => {
        const data = node.data as FolderNodeData;
        filters.selectFolder(data.folderIdentifier, isAdditive);
    };

    /**
     * Triggers the context menu for a specific node.
     *
     * @param event - The mouse event triggered by right-click.
     * @param node - The tree node being interacted with.
     */
    const handleContextMenu = (event: MouseEvent, node: TreeNode) => {
        event.preventDefault();
        setContextMenuNode(node);
        setContextMenuPosition({ coordinateX: event.clientX, coordinateY: event.clientY });
        setContextMenuOpen(true);
    };

    // --- Selection State ---
    const activeSelectionIdentifiers = createMemo(() => {
        return filters.selectedFolderIds.map(folderId => `folder-${folderId}`);
    });

    return (
        <>
            <SidebarPanel
                title="Folders"
                class="panel-fluid"
                headerActions={
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
                {folderTreeHierarchy().length > 0 ? (
                    <TreeView
                        items={folderTreeHierarchy()}
                        onSelect={handleSelect}
                        selectedIds={activeSelectionIdentifiers()}
                        onContextMenu={handleContextMenu}
                        expandedIds={tree.expandedIds}
                        onToggle={tree.toggle}
                        draggable={false}
                        dragType="FOLDER"
                        acceptedDragTypes={[]}
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
                folderIdentifier={folderToDelete()?.folderIdentifier ?? null}
                folderName={folderToDelete()?.name ?? ''}
            />

            <FolderContextMenu
                isOpen={contextMenuOpen()}
                coordinateX={contextMenuPosition().coordinateX}
                coordinateY={contextMenuPosition().coordinateY}
                node={contextMenuNode()}
                onClose={() => setContextMenuOpen(false)}
                onDelete={node => {
                    setFolderToDelete(node.data as FolderNodeData);
                    setDeleteModalOpen(true);
                }}
            />
        </>
    );
};
