import { Component, createSignal, splitProps, For, createEffect } from 'solid-js';
import { cn } from '../../../lib/utils';
import { createId } from '../../../lib/primitives/createId';
import { currentDragItem, DragItem } from '../../../core/dnd';
import { TreeViewProps } from './types';
import { TreeViewItem } from './TreeViewItem';
import './tree-view.css';

/**
 * TreeView component for displaying hierarchical navigation structures.
 *
 * This component provides a specialized UI for traversing deep hierarchies, supporting
 * single selection, custom indentation, and integrated drag-and-drop orchestration.
 * It manages root-level drop operations and delegates individual node rendering
 * to the TreeViewItem component.
 *
 * @template T - The type of business data associated with each tree node.
 * @param {TreeViewProps<T>} props - The properties for configuring the tree view.
 * @returns {JSX.Element} A reactive container element representing the hierarchical tree.
 *
 * @example
 * const itemHierarchyList = [
 *   { id: '1', label: 'Main Box', children: [{ id: '2', label: 'Inner Item' }] }
 * ];
 *
 * <TreeView
 *   items={itemHierarchyList}
 *   onSelect={(node) => console.log(node.label)}
 *   dragType="TAG"
 *   acceptedDragTypes={['TAG', 'ASSET']}
 * />
 */
export const TreeView: Component<TreeViewProps<unknown>> = props => {
    // Separate core tree data from props for clean attribute passing
    const [localProperties] = splitProps(props, [
        'items',
        'class',
        'indentSize',
        'draggable',
        'dragType',
        'acceptedDragTypes'
    ]);

    const [isRootDragOverActive, setIsRootDragOverActive] = createSignal(false);

    /** Clean up root drag active state when drag ends globally */
    createEffect(() => {
        if (!currentDragItem()) {
            setIsRootDragOverActive(false);
        }
    });

    /** Unique identifier for the tree instance to manage ARIA and DOM associations */
    const uniqueTreeIdentifier = createId('tree');

    /** Indentation fallback if not provided via props */
    const indentationPixelSize = () => localProperties.indentSize ?? 16;

    /**
     * Handles dropping an item at the root of the tree (no specific target node).
     */
    const handleRootDropOperation = async (event: DragEvent) => {
        // Only trigger drop on the root container itself or its immediate children area,
        // but not if a more specific TreeViewItem already handled it and stopped propagation.
        if (event.defaultPrevented && event.target !== event.currentTarget) {
            return;
        }

        event.preventDefault();
        event.stopPropagation();
        setIsRootDragOverActive(false);

        try {
            const rawJsonData = event.dataTransfer?.getData('application/json');
            if (rawJsonData) {
                const droppedItem: DragItem = JSON.parse(rawJsonData);

                // Check if this type is accepted at the root
                const isAccepted =
                    localProperties.acceptedDragTypes?.includes(droppedItem.type) ?? true;

                if (isAccepted && props.onDrop) {
                    props.onDrop(droppedItem, 'root', 'inside');
                }
            }
        } catch (error) {
            console.error('Tree root drop operation failed:', error);
        }
    };

    const handleRootDragOver = (event: DragOverEvent) => {
        event.preventDefault();

        // If hovering directly over an item, the item handles its own highlighting.
        // We only want the root highlight when dragging over the empty area of the tree.
        const targetElement = event.target as HTMLElement;
        const isOverItem = targetElement.closest('.ui-tree-item');
        if (isOverItem) {
            setIsRootDragOverActive(false);
            return;
        }

        const activeDragSource = currentDragItem();

        // If it's the tree's own type, it's a move (reorder), otherwise it's a copy (assignment)
        const isTypeAccepted =
            activeDragSource &&
            (localProperties.acceptedDragTypes?.includes(activeDragSource.type) ?? true);

        // Accept drops on the background area of the tree
        const isValidRootTarget = isTypeAccepted;

        if (isValidRootTarget) {
            setIsRootDragOverActive(true);
            if (event.dataTransfer) {
                // If it's the tree's own type, it's a move (reorder), otherwise it's a copy (assignment)
                event.dataTransfer.dropEffect =
                    props.dropEffect ??
                    (activeDragSource.type === localProperties.dragType ? 'move' : 'copy');
            }
        }
    };

    return (
        <div
            id={uniqueTreeIdentifier}
            class={cn(
                'ui-tree',
                isRootDragOverActive() && 'ui-tree-root-drop-active',
                localProperties.class
            )}
            role="tree"
            aria-label="Hierarchical navigation tree"
            onDragEnter={event => event.preventDefault()}
            onDragOver={handleRootDragOver}
            onDragLeave={event => {
                if (event.target === event.currentTarget) {
                    setIsRootDragOverActive(false);
                }
            }}
            onDrop={handleRootDropOperation}
        >
            <For each={localProperties.items}>
                {(treeNode, index) => (
                    <TreeViewItem
                        {...props}
                        node={treeNode}
                        depth={0}
                        treeId={uniqueTreeIdentifier}
                        indentSize={indentationPixelSize()}
                        draggable={localProperties.draggable ?? true}
                        isLast={index() === localProperties.items.length - 1}
                    />
                )}
            </For>
        </div>
    );
};

type DragOverEvent = DragEvent & { currentTarget: HTMLDivElement; target: Element };
