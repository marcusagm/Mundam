import { Component, createSignal, Show, For, JSX, createMemo } from 'solid-js';
import { cn } from '../../../lib/utils';
import { currentDragItem, DragItem } from '../../../core/dnd';
import { TreeViewItemProps, TreeNode, TreeDropPosition } from './types';
import { useTreeNavigation } from './hooks/useTreeNavigation';
import { useTreeDragDrop } from './hooks/useTreeDragDrop';
import { TreeViewToggle } from './components/TreeViewToggle';
import { TreeViewIcon } from './components/TreeViewIcon';
import { TreeViewLabel } from './components/TreeViewLabel';
import { TreeViewBadge } from './components/TreeViewBadge';
import { TreeViewInput } from './components/TreeViewInput';

/**
 * Internal presentational component for an individual tree row.
 *
 * This component handles the visual rendering of a node's content, including indentation,
 * drag-and-drop visual indicators (drop lines), icons, labels, and state indicators (selection, expansion).
 * It delegates all complex logic to the orchestrator via props.
 *
 * @param {Object} props - The row's presentational properties and event handlers.
 * @returns {JSX.Element} The rendered tree item row.
 */
const TreeViewItemRow: Component<{
    node: TreeNode<unknown>;
    depth: number;
    treeId: string;
    indentSize: number;
    isSelected: boolean;
    isExpanded: boolean;
    isEditing: boolean;
    hasChildren: boolean;
    dragDrop: ReturnType<typeof useTreeDragDrop>;
    defaultIcon?: Component;
    onSelect?: (isAdditive: boolean) => void;
    onToggle: (event: MouseEvent) => void;
    onContextMenu?: (event: MouseEvent) => void;
    onRename: (newName: string) => void;
    onEditCancel: () => void;
    onFocusChange: (isFocused: boolean) => void;
}> = props => {
    const indentationOffset = () => props.depth * props.indentSize;

    return (
        <div
            id={`${props.treeId}-item-${props.node.id}`}
            role="treeitem"
            aria-selected={props.isSelected}
            aria-expanded={props.hasChildren ? props.isExpanded : undefined}
            tabindex={props.isSelected ? 0 : -1}
            class={cn(
                'ui-tree-item',
                props.isSelected && 'ui-tree-item-selected',
                props.dragDrop.dropPosition() === 'inside' && 'ui-tree-item-drop-target',
                props.dragDrop.isDropInvalid() && 'ui-tree-item-drop-disabled',
                props.dragDrop.isDraggingSource() && 'ui-tree-item-dragging'
            )}
            draggable={!props.isEditing}
            onDragStart={event => props.dragDrop.handleDragStart(event)}
            onDragEnd={() => props.dragDrop.handleDragEnd()}
            onDragOver={event => props.dragDrop.handleDragOver(event)}
            onDragLeave={() => props.dragDrop.handleDragLeave()}
            onDragEnter={event => event.preventDefault()}
            onDrop={event => props.dragDrop.handleDrop(event)}
            onClick={event => !props.isEditing && props.onSelect?.(event.ctrlKey || event.metaKey)}
            onContextMenu={event => props.onContextMenu?.(event)}
            onFocusIn={() => props.onFocusChange(true)}
            onFocusOut={() => props.onFocusChange(false)}
        >
            <Show when={props.dragDrop.dropPosition() === 'before'}>
                <div
                    class="ui-tree-drop-line ui-tree-drop-line-before"
                    style={{ left: `${indentationOffset()}px` }}
                />
            </Show>
            <Show when={props.dragDrop.dropPosition() === 'after'}>
                <div
                    class="ui-tree-drop-line ui-tree-drop-line-after"
                    style={{ left: `${indentationOffset()}px` }}
                />
            </Show>

            <div class="ui-tree-item-content" style={{ 'margin-left': `${indentationOffset()}px` }}>
                <Show when={props.hasChildren} fallback={<span class="ui-tree-toggle-spacer" />}>
                    <TreeViewToggle isExpanded={props.isExpanded} onClick={props.onToggle} />
                </Show>

                <Show when={props.node.icon || props.defaultIcon}>
                    <TreeViewIcon
                        icon={(props.node.icon || props.defaultIcon)!}
                        color={props.node.iconColor}
                    />
                </Show>

                <Show
                    when={props.isEditing}
                    fallback={
                        <>
                            <TreeViewLabel text={props.node.label} />
                            <Show when={props.node.badge}>
                                <TreeViewBadge children={props.node.badge} />
                            </Show>
                        </>
                    }
                >
                    <TreeViewInput
                        value={props.node.label}
                        onCommit={props.onRename}
                        onCancel={props.onEditCancel}
                    />
                </Show>
            </div>
        </div>
    );
};

/**
 * Functional component for rendering an individual node and its nested children within a TreeView.
 *
 * This component acts as an orchestrator, managing the node's internal state (expansion, focus),
 * registering keyboard navigation via the `useTreeNavigation` hook, and configuring
 * drag-and-drop logic via the `useTreeDragDrop` hook. It recursively renders child nodes
 * when expanded.
 *
 * @template T - The type of business data associated with the tree node.
 * @param {TreeViewItemProps<T>} props - The properties for the tree item.
 * @returns {JSX.Element} The rendered tree item and its potential nested group.
 */
export const TreeViewItem: Component<TreeViewItemProps<unknown>> = props => {
    const [localExpansionState, setLocalExpansionState] = createSignal(false);
    const [isFocused, setIsFocused] = createSignal(false);
    const isExpanded = createMemo(() =>
        props.expandedIds ? props.expandedIds.has(props.node.id) : localExpansionState()
    );
    const isEditing = createMemo(() => props.editingId === props.node.id);
    const hasChildren = createMemo(() => !!(props.node.children && props.node.children.length > 0));
    const isSelected = createMemo(
        () => props.selectedIds?.some(id => String(id) === String(props.node.id)) ?? false
    );
    const indentationPixelSize = () => props.indentSize ?? 16;
    const indentationOffset = () => props.depth * indentationPixelSize();
    const navigationOptions = {
        node: () => props.node,
        isEditing,
        hasChildren,
        isExpanded,
        isFocused,
        onSelect: () => props.onSelect?.(props.node),
        onToggle: (state: boolean) =>
            props.onToggle ? props.onToggle(props.node.id) : setLocalExpansionState(state),
        onEditCancel: () => props.onEditCancel?.()
    };
    useTreeNavigation(navigationOptions);

    const dragDropOptions = {
        node: () => props.node as TreeNode<unknown>,
        isEnabled: () => props.draggable ?? false,
        isEditing,
        dragType: () => props.dragType,
        acceptedDragTypes: () => props.acceptedDragTypes,
        isValidDrop: (dragged: DragItem, target: TreeNode<unknown>) =>
            props.isValidDrop?.(dragged, target) ?? true,
        onDrop: (item: DragItem, targetId: string | number, position: TreeDropPosition) =>
            props.onDrop?.(item, targetId, position)
    };
    const dragDrop = useTreeDragDrop(dragDropOptions);

    return (
        <div
            class={cn(
                'ui-tree-item-container',
                props.depth > 0 && 'ui-tree-has-guide',
                props.isLast && 'ui-tree-last-item'
            )}
            style={
                {
                    '--tree-indent': `${indentationOffset()}px`,
                    '--guide-pos': `${indentationOffset() - indentationPixelSize() / 2 + 4}px`
                } as JSX.CSSProperties
            }
        >
            <TreeViewItemRow
                node={props.node}
                depth={props.depth}
                treeId={props.treeId}
                indentSize={indentationPixelSize()}
                isSelected={isSelected()}
                isExpanded={isExpanded()}
                isEditing={isEditing()}
                hasChildren={hasChildren()}
                dragDrop={dragDrop}
                defaultIcon={props.defaultIcon}
                onSelect={isAdditive => props.onSelect?.(props.node, isAdditive)}
                onToggle={event => {
                    event.stopPropagation();
                    if (hasChildren()) {
                        if (props.onToggle) props.onToggle(props.node.id);
                        else setLocalExpansionState(!localExpansionState());
                    }
                }}
                onContextMenu={event => props.onContextMenu?.(event, props.node)}
                onRename={newName => {
                    if (newName && newName !== props.node.label)
                        props.onRename?.(props.node, newName);
                    else props.onEditCancel?.();
                }}
                onEditCancel={() => props.onEditCancel?.()}
                onFocusChange={setIsFocused}
            />

            <Show when={isExpanded() && hasChildren()}>
                <div
                    id={`${props.treeId}-group-${props.node.id}`}
                    role="group"
                    class="ui-tree-group"
                    aria-label={`Children of ${props.node.label}`}
                    onDragOver={event => {
                        const dragging = currentDragItem();
                        if (
                            dragging &&
                            (props.acceptedDragTypes?.includes(dragging.type) ?? true)
                        ) {
                            event.preventDefault();
                            event.stopPropagation();
                            if (event.dataTransfer) {
                                event.dataTransfer.dropEffect =
                                    dragging.type === props.dragType ? 'move' : 'copy';
                            }
                        }
                    }}
                    onDrop={event => {
                        event.preventDefault();
                        event.stopPropagation();
                        /**
                         * Dropping on the group container itself (not a specific child row)
                         * is interpreted as a request to move the item INSIDE this parent node.
                         */
                        const rawJsonData = event.dataTransfer?.getData('application/json');
                        if (rawJsonData && props.onDrop) {
                            const droppedItem: DragItem = JSON.parse(rawJsonData);
                            props.onDrop(droppedItem, props.node.id, 'inside');
                        }
                    }}
                >
                    <For each={props.node.children}>
                        {(childNode, childIndex) => (
                            <TreeViewItem
                                {...props}
                                node={childNode}
                                depth={props.depth + 1}
                                isLast={childIndex() === (props.node.children?.length ?? 0) - 1}
                            />
                        )}
                    </For>
                </div>
            </Show>
        </div>
    );
};
