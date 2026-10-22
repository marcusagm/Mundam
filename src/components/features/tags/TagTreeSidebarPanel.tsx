import { Button } from '../../ui';
import { Component, createMemo, createSignal, untrack } from 'solid-js';
import { Bookmark, Plus } from 'lucide-solid';
import {
    useMetadata,
    useFilters,
    useNotification,
    useTree,
    useDndHandlers
} from '../../../core/hooks';
import { TreeView, TreeNode } from '../../ui/TreeView';
import { SidebarPanel } from '../../ui/SidebarPanel';
import { CountBadge } from '../../ui/CountBadge';
import { dndRegistry, DragItem } from '../../../core/dnd';
import { TagDomainService } from '../../../core/services/TagDomainService';
import { TagContextMenu } from './TagContextMenu';
import { TagDeleteModal } from './TagDeleteModal';

/**
 * Sidebar panel for managing the hierarchical tag system.
 * Integrates metadata state with the pure TreeView component.
 */
export const TagTreeSidebarPanel: Component = () => {
    const metadata = useMetadata();
    const filters = useFilters();
    const notification = useNotification();
    const tree = useTree('tags');

    // --- Component State ---
    const [isTagHeaderDragOver, setIsTagHeaderDragOver] = createSignal(false);
    const [contextMenuOpen, setContextMenuOpen] = createSignal(false);
    const [contextMenuPosition, setContextMenuPosition] = createSignal({
        coordinateX: 0,
        coordinateY: 0
    });
    const [contextMenuNode, setContextMenuNode] = createSignal<TreeNode | null>(null);

    const [editingId, setEditingId] = createSignal<string | null>(null);

    const [deleteModalOpen, setDeleteModalOpen] = createSignal(false);
    const [nodeToDelete, setNodeToDelete] = createSignal<TreeNode | null>(null);

    // --- Tree Construction ---
    /**
     * Identifies structural changes (hierarchy, order, naming) to avoid rebuilding
     * the entire tree when only visual properties like colors update.
     */
    const tagTreeStructuralHash = createMemo(() => {
        const allTags = metadata.tags || [];
        // We include name because it often dictates visual order or grouping
        return allTags
            .map(tag => `${tag.id}-${tag.parent_id}-${tag.order_index}-${tag.name}`)
            .join('|');
    });

    const tagTreeHierarchy = createMemo(() => {
        // Depend on structural changes only
        tagTreeStructuralHash();

        return untrack(() => {
            const allTags = metadata.tags || [];
            const nodeMap = new Map<string, TreeNode>();
            const rootNodes: TreeNode[] = [];

            // Phase 1: Create all nodes with reactive getters
            allTags.forEach(tag => {
                const tagId = tag.id;
                nodeMap.set(tagId, {
                    id: tagId,
                    get label() {
                        const current = metadata.tags.find(t => t.id === tagId);
                        return current?.name || '';
                    },
                    children: [],
                    data: tag,
                    icon: Bookmark,
                    get iconColor() {
                        const current = metadata.tags.find(t => t.id === tagId);
                        return current?.color || undefined;
                    },
                    get badge() {
                        return (
                            <CountBadge
                                showZero={true}
                                count={metadata.stats.tag_counts.get(tagId) || 0}
                            />
                        );
                    }
                });
            });

            // Phase 2: Build hierarchy relationships
            allTags.forEach(tag => {
                if (tag.parent_id && nodeMap.has(tag.parent_id)) {
                    nodeMap.get(tag.parent_id)!.children!.push(nodeMap.get(tag.id)!);
                } else {
                    rootNodes.push(nodeMap.get(tag.id)!);
                }
            });

            return rootNodes;
        });
    });

    // --- Domain Logic Handlers ---
    const getUniqueTagName = (baseName: string) => {
        const existingTags = metadata.tags || [];
        let name = TagDomainService.normalizeName(baseName);
        let counter = 1;
        while (existingTags.some(tag => tag.name === name)) {
            name = `${TagDomainService.normalizeName(baseName)} (${counter})`;
            counter++;
        }
        return name;
    };

    const handleCreateTag = async () => {
        if (editingId() !== null) return;
        const name = getUniqueTagName('New Tag');
        const result = await metadata.createTag(name);

        if (result.success && result.data) {
            // Give the tree a moment to render the new node before entering edit mode
            setTimeout(() => {
                setEditingId(result.data);
            }, 100);
        } else {
            notification.error('Failed to Create Tag');
        }
    };

    const handleCreateChildTag = async (parentId: string) => {
        tree.setExpanded(parentId, true);
        const name = getUniqueTagName('New Tag');
        const result = await metadata.createTag(name, parentId);

        if (result.success && result.data) {
            // Give the tree a moment to render the new node before entering edit mode
            setTimeout(() => {
                setEditingId(result.data);
            }, 100);
        } else {
            notification.error('Failed to Create Child Tag');
        }
    };

    const handleRenameTag = async (node: TreeNode, newName: string) => {
        const normalized = TagDomainService.normalizeName(newName);
        if (!normalized || normalized === node.label) {
            setEditingId(null);
            return;
        }

        const oldName = node.label;
        const isPlaceholder = /^New Tag( \(\d+\))?$/.test(oldName);

        const result = await metadata.updateTag(String(node.id), normalized);

        if (result.success) {
            if (isPlaceholder) {
                notification.success('Tag Created', `Created tag "${normalized}"`);
            } else {
                notification.success('Tag Renamed', `Changed "${oldName}" to "${normalized}"`);
            }
        } else {
            notification.error('Failed to Rename Tag');
        }
        setEditingId(null);
    };

    /**
     * Triggers the context menu for a specific node.
     *
     * @param {MouseEvent} event - The mouse event triggered by right-click.
     * @param {TreeNode} node - The tree node being interacted with.
     */
    const handleContextMenu = (event: MouseEvent, node: TreeNode) => {
        event.preventDefault();
        setContextMenuPosition({ coordinateX: event.clientX, coordinateY: event.clientY });
        setContextMenuNode(node);
        setContextMenuOpen(true);
    };

    const { handleDrop } = useDndHandlers();

    /**
     * Domain-specific validation for tag drops.
     * Prevents dropping a tag into its own descendant hierarchy.
     */
    const isValidTagDrop = (draggedItem: DragItem, targetNode: TreeNode): boolean => {
        if (draggedItem.type !== 'TAG') return true;

        const draggedId = String(draggedItem.payload.id);
        const targetId = String(targetNode.id);

        /** Recursive check for descendants */
        const isDescendant = (node: TreeNode, searchId: string): boolean => {
            if (node.children) {
                for (const child of node.children) {
                    if (String(child.id) === searchId) return true;
                    if (isDescendant(child, searchId)) return true;
                }
            }
            return false;
        };

        // Find the dragged node in our local tree to check its children
        const findDraggedNode = (nodes: TreeNode[]): TreeNode | null => {
            for (const node of nodes) {
                if (String(node.id) === draggedId) return node;
                if (node.children) {
                    const found = findDraggedNode(node.children);
                    if (found) return found;
                }
            }
            return null;
        };

        const node = findDraggedNode(tagTreeHierarchy());
        return !node || !isDescendant(node, targetId);
    };

    /**
     * Handles drops on the root area of the tag panel.
     * Allows tags to be moved back to the root level.
     *
     * @param {DragEvent} event - The browser drop event.
     */
    const handleTagPaneRootDrop = async (event: DragEvent) => {
        event.preventDefault();
        setIsTagHeaderDragOver(false);
        try {
            const rawJsonData = event.dataTransfer?.getData('application/json');
            if (rawJsonData) {
                const droppedItem: DragItem = JSON.parse(rawJsonData);
                if (droppedItem.type === 'TAG') {
                    await handleDrop(droppedItem, 'root', 'TAG');
                }
            }
        } catch (error) {
            console.error('Root tag drop failed:', error);
        }
    };

    return (
        <SidebarPanel
            title="Tags"
            class="panel-fluid"
            headerActions={
                <Button variant="ghost" size="icon-xs" title="Create Tag" onClick={handleCreateTag}>
                    <Plus size={14} />
                </Button>
            }
            contentClass={isTagHeaderDragOver() ? 'drag-over' : ''}
            onDragOver={(event: DragEvent) => {
                const strategy = dndRegistry.get('TAG');
                if (strategy) {
                    event.preventDefault();
                    if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';
                    setIsTagHeaderDragOver(true);
                }
            }}
            onDragLeave={() => setIsTagHeaderDragOver(false)}
            onDrop={handleTagPaneRootDrop}
        >
            <TreeView
                items={tagTreeHierarchy()}
                onSelect={(node, isAdditive) => filters.selectTag(String(node.id), isAdditive)}
                selectedIds={filters.selectedTags}
                onContextMenu={handleContextMenu}
                editingId={editingId()}
                onRename={handleRenameTag}
                onEditCancel={() => setEditingId(null)}
                expandedIds={tree.expandedIds}
                onToggle={tree.toggle}
                draggable={true}
                dragType="TAG"
                acceptedDragTypes={['TAG', 'ASSET']}
                isValidDrop={isValidTagDrop}
                onDrop={(item, targetId, position) => handleDrop(item, targetId, 'TAG', position)}
            />

            <TagContextMenu
                coordinateX={contextMenuPosition().coordinateX}
                coordinateY={contextMenuPosition().coordinateY}
                isOpen={contextMenuOpen()}
                node={contextMenuNode()}
                onClose={() => setContextMenuOpen(false)}
                onAddChild={handleCreateChildTag}
                onRename={id => setEditingId(id)}
                onDelete={node => {
                    setNodeToDelete(node);
                    setDeleteModalOpen(true);
                }}
            />

            <TagDeleteModal
                isOpen={deleteModalOpen()}
                onClose={() => setDeleteModalOpen(false)}
                node={nodeToDelete()}
            />
        </SidebarPanel>
    );
};
