import {
    Component,
    For,
    createMemo,
    createSignal,
    Show,
    createEffect,
    splitProps,
    JSX
} from 'solid-js';
import { ChevronRight, ChevronDown } from 'lucide-solid';
import { Dynamic } from 'solid-js/web';
import { cn } from '../../lib/utils';
import { createId } from '../../lib/primitives/createId';
import { dndRegistry, setDragItem, currentDragItem } from '../../core/dnd';
import { useMetadata } from '../../core/hooks';
import { useInput, SCOPE_PRIORITIES } from '../../core/input';
import './tree-view.css';

export interface TreeNode {
    /** Unique identifier */
    id: string | number;
    /** Display label */
    label: string;
    /** Child nodes */
    children?: TreeNode[];
    /** Additional data attached to this node */
    data?: unknown;
    /** Icon component */
    icon?: Component<{ size?: number | string; color?: string; fill?: string; stroke?: string }>;
    /** Icon color (CSS color value) */
    iconColor?: string;
    /** Badge element to display on the right */
    badge?: JSX.Element;
}

export interface TreeViewProps {
    /** Tree data */
    items: TreeNode[];
    /** Callback when a node is selected */
    onSelect?: (node: TreeNode) => void;
    /** Callback for context menu */
    onContextMenu?: (e: MouseEvent, node: TreeNode) => void;
    /** Currently selected node IDs */
    selectedIds?: (string | number)[];
    /** ID of node currently being edited */
    editingId?: string | number | null;
    /** Callback when node is renamed */
    onRename?: (node: TreeNode, newName: string) => void;
    /** Callback when editing is cancelled */
    onEditCancel?: () => void;
    /** Default icon for all nodes */
    defaultIcon?: Component<{
        size?: number | string;
        color?: string;
        fill?: string;
        stroke?: string;
    }>;
    /** Set of expanded node IDs (controlled) */
    expandedIds?: Set<string | number>;
    /** Callback when node expansion is toggled */
    onToggle?: (id: string | number) => void;
    /** Callback when node is moved via drag-drop */
    onMove?: (draggedId: string | number, targetId: string | number) => void;
    /** Additional CSS class */
    class?: string;
    /** Indentation per level in pixels */
    indentSize?: number;
    /** Whether drag-and-drop is enabled (default: true) */
    draggable?: boolean;
}

interface TreeViewItemProps {
    node: TreeNode;
    depth: number;
    onSelect?: (node: TreeNode) => void;
    onContextMenu?: (e: MouseEvent, node: TreeNode) => void;
    selectedIds?: (string | number)[];
    editingId?: string | number | null;
    onRename?: (node: TreeNode, newName: string) => void;
    onEditCancel?: () => void;
    defaultIcon?: Component<{
        size?: number | string;
        color?: string;
        fill?: string;
        stroke?: string;
    }>;
    expandedIds?: Set<string | number>;
    onToggle?: (id: string | number) => void;
    onMove?: (draggedId: string | number, targetId: string | number) => void;
    treeId: string;
    indentSize: number;
    isLast?: boolean;
    draggable: boolean;
}

/**
 * TreeViewItem - Internal component for rendering individual tree nodes.
 */
const TreeViewItem: Component<TreeViewItemProps> = props => {
    const metadata = useMetadata();
    const [localExpanded, setLocalExpanded] = createSignal(false);
    const [dropPosition, setDropPosition] = createSignal<'before' | 'inside' | 'after' | null>(
        null
    );
    const input = useInput();

    let inputRef: HTMLInputElement | undefined;
    let isEditingScopeActive = false;

    const isExpanded = () =>
        props.expandedIds ? props.expandedIds.has(props.node.id) : localExpanded();

    const isEditing = () => props.editingId === props.node.id;
    const hasChildren = () => props.node.children && props.node.children.length > 0;

    const isSelected = () =>
        props.selectedIds?.some(id => String(id) === String(props.node.id)) ?? false;

    const itemId = `${props.treeId}-item-${props.node.id}`;
    const groupId = `${props.treeId}-group-${props.node.id}`;

    // Calculate indent
    const indent = () => props.depth * props.indentSize;

    // Drag/Drop validation
    const validationState = createMemo(() => {
        const dragItem = currentDragItem();
        if (!dragItem || dragItem.type !== 'TAG') return { valid: true };

        const draggedId = Number(dragItem.payload.id);
        const targetId = Number(props.node.id);

        if (draggedId === targetId) return { valid: false };

        const isDescendant = (parentId: number, childId: number): boolean => {
            let current = metadata.tags.find(t => t.id === childId);
            while (current && current.parent_id) {
                if (current.parent_id === parentId) return true;
                const nextParentId = current.parent_id;
                current = metadata.tags.find(t => t.id === nextParentId);
            }
            return false;
        };

        if (isDescendant(draggedId, targetId)) return { valid: false };

        return { valid: true };
    });

    const isDraggingSource = () =>
        currentDragItem()?.type === 'TAG' &&
        Number(currentDragItem()?.payload.id) === Number(props.node.id);

    const isInvalid = () => !validationState().valid && currentDragItem()?.type === 'TAG';

    // Drag handlers
    const handleDragStart = (e: DragEvent) => {
        if (!props.draggable) return;
        e.stopPropagation();
        if (!isEditing() && e.dataTransfer) {
            const data = { type: 'TAG', payload: { id: props.node.id } };
            setDragItem(data as any);
            e.dataTransfer.effectAllowed = 'move';
            e.dataTransfer.setData('application/json', JSON.stringify(data));
        }
    };

    const handleDragEnd = () => {
        setDragItem(null);
        setDropPosition(null);
    };

    const handleDragOver = (e: DragEvent) => {
        e.preventDefault();

        const validation = validationState();
        const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
        const relY = e.clientY - rect.top;
        const height = rect.height;
        const threshold = height * 0.25;

        let pos: 'before' | 'inside' | 'after' = 'inside';
        if (relY < threshold) pos = 'before';
        else if (relY > height - threshold) pos = 'after';

        if (!props.draggable || !validation.valid) {
            e.dataTransfer!.dropEffect = 'none';
            setDropPosition(null);
            return;
        }

        const dragItem = currentDragItem();
        if (dragItem?.type === 'TAG') {
            setDropPosition(pos);
            e.dataTransfer!.dropEffect = 'move';
        } else if (dragItem?.type === 'IMAGE') {
            setDropPosition('inside');
            e.dataTransfer!.dropEffect = 'copy';
        }
    };

    const handleDragLeave = () => {
        setDropPosition(null);
    };

    const handleDrop = async (e: DragEvent) => {
        e.preventDefault();
        e.stopPropagation();
        const pos = dropPosition();
        setDropPosition(null);

        if (!validationState().valid) return;

        try {
            const json = e.dataTransfer?.getData('application/json');
            if (json) {
                const item = JSON.parse(json);
                const strategy = dndRegistry.get('TAG');
                if (strategy && strategy.accepts(item)) {
                    await (strategy as any).onDrop(item, props.node.id, pos || 'inside');
                }
            }
        } catch (err) {
            console.error('Drop failed', err);
        } finally {
            setDragItem(null);
        }
    };

    // Focus input when editing
    createEffect(() => {
        if (isEditing() && inputRef) {
            inputRef.focus();
            inputRef.select();
        }
    });

    const handleToggle = (e: MouseEvent) => {
        e.stopPropagation();
        if (hasChildren()) {
            if (props.onToggle) {
                props.onToggle(props.node.id);
            } else {
                setLocalExpanded(!localExpanded());
            }
        }
    };

    const handleKeyDown = (e: KeyboardEvent) => {
        switch (e.key) {
            case 'Enter':
                if (isEditing()) {
                    (e.currentTarget as HTMLInputElement).blur();
                } else {
                    props.onSelect?.(props.node);
                }
                break;
            case 'Escape':
                if (isEditing()) {
                    props.onEditCancel?.();
                }
                break;
            case 'ArrowRight':
                if (hasChildren() && !isExpanded()) {
                    e.preventDefault();
                    if (props.onToggle) {
                        props.onToggle(props.node.id);
                    } else {
                        setLocalExpanded(true);
                    }
                }
                break;
            case 'ArrowLeft':
                if (hasChildren() && isExpanded()) {
                    e.preventDefault();
                    if (props.onToggle) {
                        props.onToggle(props.node.id);
                    } else {
                        setLocalExpanded(false);
                    }
                }
                break;
        }
    };

    const handleInputKeyDown = (e: KeyboardEvent) => {
        e.stopPropagation();
        if (e.key === 'Enter') {
            e.preventDefault();
            (e.currentTarget as HTMLInputElement).blur();
        } else if (e.key === 'Escape') {
            e.preventDefault();
            props.onEditCancel?.();
        }
    };

    const handleInputFocus = () => {
        if (!isEditingScopeActive) {
            input.pushScope('editing', SCOPE_PRIORITIES.editing, true);
            isEditingScopeActive = true;
        }
    };

    const handleBlur = () => {
        if (isEditingScopeActive) {
            input.popScope('editing');
            isEditingScopeActive = false;
        }
        if (inputRef) {
            const val = inputRef.value.trim();
            if (val && val !== props.node.label) {
                props.onRename?.(props.node, val);
            } else {
                props.onEditCancel?.();
            }
        }
    };

    return (
        <div
            class={cn(
                'ui-tree-item-container',
                props.depth > 0 && 'ui-tree-has-guide',
                props.isLast && 'ui-tree-last-item'
            )}
            style={
                {
                    '--tree-indent': `${indent()}px`,
                    '--guide-pos': `${indent() - props.indentSize / 2 + 4}px`
                } as JSX.CSSProperties
            }
        >
            <div
                id={itemId}
                role="treeitem"
                aria-selected={isSelected()}
                aria-expanded={hasChildren() ? isExpanded() : undefined}
                aria-owns={hasChildren() ? groupId : undefined}
                tabindex={isSelected() ? 0 : -1}
                class={cn(
                    'ui-tree-item',
                    isSelected() && 'ui-tree-item-selected',
                    props.draggable && dropPosition() === 'inside' && 'ui-tree-item-drop-target',
                    isInvalid() && 'ui-tree-item-drop-disabled',
                    isDraggingSource() && 'ui-tree-item-dragging'
                )}
                draggable={props.draggable && !isEditing()}
                onDragStart={handleDragStart}
                onDragEnd={handleDragEnd}
                onDragEnter={e => e.preventDefault()}
                onDragOver={handleDragOver}
                onDragLeave={handleDragLeave}
                onDrop={handleDrop}
                onClick={() => !isEditing() && props.onSelect?.(props.node)}
                onContextMenu={e => {
                    e.preventDefault();
                    props.onContextMenu?.(e, props.node);
                }}
                onKeyDown={handleKeyDown}
            >
                {/* Drop indicator lines */}
                <Show when={dropPosition() === 'before'}>
                    <div
                        class="ui-tree-drop-line ui-tree-drop-line-before"
                        style={{ left: `${indent()}px` }}
                    />
                </Show>
                <Show when={dropPosition() === 'after'}>
                    <div
                        class="ui-tree-drop-line ui-tree-drop-line-after"
                        style={{ left: `${indent()}px` }}
                    />
                </Show>

                {/* Item content with indent */}
                <div class="ui-tree-item-content" style={{ 'margin-left': `${indent()}px` }}>
                    {/* Toggle button - only shown when has children */}
                    <Show when={hasChildren()} fallback={<span class="ui-tree-toggle-spacer" />}>
                        <button
                            type="button"
                            class="ui-tree-toggle"
                            onClick={handleToggle}
                            aria-label={isExpanded() ? 'Collapse' : 'Expand'}
                            tabindex={-1}
                        >
                            <Show when={isExpanded()} fallback={<ChevronRight size={12} />}>
                                <ChevronDown size={12} />
                            </Show>
                        </button>
                    </Show>

                    {/* Icon */}
                    <Show when={props.node.icon || props.defaultIcon}>
                        <span
                            class="ui-tree-icon"
                            style={{ color: props.node.iconColor || 'var(--text-secondary)' }}
                            aria-hidden="true"
                        >
                            <Dynamic component={props.node.icon || props.defaultIcon} size={14} />
                        </span>
                    </Show>

                    {/* Label or edit input */}
                    <Show
                        when={isEditing()}
                        fallback={
                            <>
                                <span class="ui-tree-label">{props.node.label}</span>
                                <Show when={props.node.badge}>
                                    <span class="ui-tree-badge">{props.node.badge}</span>
                                </Show>
                            </>
                        }
                    >
                        <input
                            ref={inputRef}
                            type="text"
                            class="ui-tree-input"
                            value={props.node.label}
                            onClick={e => e.stopPropagation()}
                            onKeyDown={handleInputKeyDown}
                            onFocus={handleInputFocus}
                            onBlur={handleBlur}
                            onInput={() => {}}
                            aria-label="Rename item"
                        />
                    </Show>
                </div>
            </div>

            {/* Children */}
            <Show when={isExpanded() && hasChildren()}>
                <div
                    id={groupId}
                    role="group"
                    class="ui-tree-group"
                    aria-label={`Children of ${props.node.label}`}
                >
                    <For each={props.node.children}>
                        {(child, index) => (
                            <TreeViewItem
                                node={child}
                                depth={props.depth + 1}
                                onSelect={props.onSelect}
                                onContextMenu={props.onContextMenu}
                                selectedIds={props.selectedIds}
                                editingId={props.editingId}
                                onRename={props.onRename}
                                onEditCancel={props.onEditCancel}
                                defaultIcon={props.defaultIcon}
                                expandedIds={props.expandedIds}
                                onToggle={props.onToggle}
                                onMove={props.onMove}
                                treeId={props.treeId}
                                indentSize={props.indentSize}
                                draggable={props.draggable}
                                isLast={index() === (props.node.children?.length ?? 0) - 1}
                            />
                        )}
                    </For>
                </div>
            </Show>
        </div>
    );
};

/**
 * TreeView component for displaying hierarchical data.
 * Supports selection, keyboard navigation, drag-and-drop reordering,
 * inline editing, and visual hierarchy guides.
 *
 * @example
 * const items: TreeNode[] = [
 *   { id: 1, label: "Folder", children: [
 *     { id: 2, label: "File 1" },
 *     { id: 3, label: "File 2" },
 *   ]},
 * ];
 *
 * <TreeView
 *   items={items}
 *   onSelect={(node) => console.log("Selected:", node.id)}
 * />
 */
export const TreeView: Component<TreeViewProps> = props => {
    const [local] = splitProps(props, [
        'items',
        'onSelect',
        'onContextMenu',
        'selectedIds',
        'editingId',
        'onRename',
        'onEditCancel',
        'defaultIcon',
        'expandedIds',
        'onToggle',
        'onMove',
        'class',
        'indentSize',
        'draggable'
    ]);

    const [isDragOver, setIsDragOver] = createSignal(false);
    const treeId = createId('tree');
    const indentSize = () => local.indentSize ?? 16;

    const handleRootDrop = async (e: DragEvent) => {
        if (e.target !== e.currentTarget) return;
        e.preventDefault();
        setIsDragOver(false);

        try {
            const json = e.dataTransfer?.getData('application/json');
            if (json) {
                const item = JSON.parse(json);
                const strategy = dndRegistry.get('TAG');
                if (strategy && item.type === 'TAG') {
                    await strategy.onDrop(item, 'root');
                }
            }
        } catch (err) {
            console.error('Root drop failed', err);
        }
    };

    return (
        <div
            id={treeId}
            class={cn('ui-tree', isDragOver() && 'ui-tree-root-drop-active', local.class)}
            role="tree"
            aria-label="Tree navigation"
            onDragEnter={e => e.preventDefault()}
            onDragOver={e => {
                e.preventDefault();
                const dragItem = currentDragItem();
                if (e.target === e.currentTarget && dragItem && dragItem.type === 'TAG') {
                    setIsDragOver(true);
                    e.dataTransfer!.dropEffect = 'move';
                }
            }}
            onDragLeave={e => {
                if (e.target === e.currentTarget) setIsDragOver(false);
            }}
            onDrop={handleRootDrop}
        >
            <For each={local.items}>
                {(node, index) => (
                    <TreeViewItem
                        node={node}
                        depth={0}
                        onSelect={local.onSelect}
                        onContextMenu={local.onContextMenu}
                        selectedIds={local.selectedIds}
                        editingId={local.editingId}
                        onRename={local.onRename}
                        onEditCancel={local.onEditCancel}
                        defaultIcon={local.defaultIcon}
                        expandedIds={local.expandedIds}
                        onToggle={local.onToggle}
                        onMove={local.onMove}
                        treeId={treeId}
                        indentSize={indentSize()}
                        draggable={local.draggable ?? true}
                        isLast={index() === local.items.length - 1}
                    />
                )}
            </For>
        </div>
    );
};
