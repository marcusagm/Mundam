import { Button } from '../../ui';
import { Component, createMemo, createSignal, untrack } from 'solid-js';
import { Folder as FolderIcon, Plus } from 'lucide-solid';
import { useMetadata, useFilters, useNotification, useLibrary, useTree } from '../../../core/hooks';
import { TreeView, TreeNode } from '../../ui/TreeView';
import { SidebarPanel } from '../../ui/SidebarPanel';
import { CountBadge } from '../../ui/CountBadge';
import { FolderDeleteModal } from './FolderDeleteModal';
import { FolderContextMenu } from './FolderContextMenu';
import { FolderActionsDialog, FolderDialogTarget } from './FolderActionsDialog';
import { resolveFolderColor } from './folderPresentation';
import { useFolderTreeActions } from './useFolderTreeActions';
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
    const actions = useFolderTreeActions();
    const [dialogTarget, setDialogTarget] = createSignal<FolderDialogTarget | null>(null);

    const [deleteModalOpen, setDeleteModalOpen] = createSignal(false);
    const [folderToDelete, setFolderToDelete] = createSignal<FolderNodeData | null>(null);
    const [contextMenuOpen, setContextMenuOpen] = createSignal(false);
    const [contextMenuPosition, setContextMenuPosition] = createSignal({
        coordinateX: 0,
        coordinateY: 0
    });
    const [contextMenuNode, setContextMenuNode] = createSignal<TreeNode | null>(null);

    const folderTreeStructure = createMemo(() =>
        JSON.stringify(metadata.locations.map(folder => [folder.id, folder.parent_id, folder.name]))
    );
    const folderTreeHierarchy = createMemo(() => {
        folderTreeStructure();
        return untrack(() => {
            const allFolders = metadata.locations || [];

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
                    icon: FolderIcon,
                    get iconColor() {
                        return resolveFolderColor(folder.id, metadata.locations);
                    },
                    get badge() {
                        return (
                            <CountBadge
                                showZero={true}
                                count={
                                    (filters.folderRecursiveView
                                        ? metadata.stats.folder_counts_recursive
                                        : metadata.stats.folder_counts
                                    ).get(folder.id) || 0
                                }
                                variant="secondary"
                            />
                        );
                    }
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
                        editingId={actions.editingId()}
                        onRename={actions.handleRename}
                        onEditCancel={() => void actions.handleEditCancel()}
                        draggable={true}
                        dragType="FOLDER"
                        dropMode="inside"
                        dropEffect="move"
                        acceptedDragTypes={['ASSET', 'FOLDER']}
                        isValidDrop={actions.isValidDrop}
                        onDrop={actions.handleDrop}
                    />
                ) : (
                    <div class="sidebar-empty-state">
                        <p>No folders linked</p>
                        <p class="empty-hint">Click + to add a folder</p>
                    </div>
                )}
            </SidebarPanel>

            <FolderActionsDialog target={dialogTarget()} onClose={() => setDialogTarget(null)} />

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
                onRename={actions.setEditingId}
                onAddChild={actions.handleCreateChild}
                onAction={(action, folderId) => setDialogTarget({ action, folderId })}
                onDelete={node => {
                    setFolderToDelete(node.data as FolderNodeData);
                    setDeleteModalOpen(true);
                }}
            />
        </>
    );
};
