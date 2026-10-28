import { createSignal, createMemo } from 'solid-js';
import { currentDragItem, setDragItem, DragItem } from '../../../../core/dnd';
import { TreeNode, TreeDropPosition } from '../types';

/**
 * Options for configuring the drag-and-drop behavior for a tree node.
 */
interface TreeDragDropOptions {
    /** Accessor function that returns the node currently being targeted for drag/drop operations. */
    node: () => TreeNode<unknown>;
    /** Accessor function that returns whether drag-and-drop interactions are enabled for this node. */
    isEnabled: () => boolean;
    /** Accessor function that returns whether the node is in rename/edit mode, which disables dragging. */
    isEditing: () => boolean;
    /** Accessor function that returns the specific drag type identifier for this tree (e.g., 'TAG'). */
    dragType: () => string | undefined;
    /** Accessor function that returns the list of external drag types this node accepts. */
    acceptedDragTypes: () => string[] | undefined;
    /** Function to perform custom business logic validation for a specific drop operation. */
    isValidDrop: (dragged: DragItem, target: TreeNode<unknown>) => boolean;
    /** Callback function invoked when a drop operation is completed. */
    onDrop?: (item: DragItem, targetId: string | number, position: TreeDropPosition) => void;
}

/**
 * Custom hook for managing generic drag-and-drop state and event handlers for a tree item.
 *
 * This hook encapsulates the browser Drag-and-Drop API logic, including identifying valid
 * drop positions ('before', 'inside', 'after') and coordinating with the global DnD registry.
 * It is fully decoupled from specific domain types like tags or folders.
 *
 * @param {TreeDragDropOptions} dragDropOptions - The configuration and state accessors for the tree node.
 * @returns {GenericDropState} Accessors and handlers for managing the item's drag-and-drop lifecycle.
 */
export const useTreeDragDrop = (dragDropOptions: TreeDragDropOptions) => {
    const [dropPosition, setDropPosition] = createSignal<TreeDropPosition | null>(null);

    /**
     * Reactive memo that evaluates the validity of the current drag-and-drop operation.
     * It checks for type compatibility, prevents self-dropping, and respects custom domain rules.
     *
     * @returns {Object} An object containing the current validation status.
     */
    const validationStatus = createMemo(() => {
        const draggingItem = currentDragItem();
        if (!draggingItem) {
            return { isValid: true };
        }

        // 1. Type validation
        const acceptedTypes = dragDropOptions.acceptedDragTypes();
        const isTypeAccepted = acceptedTypes ? acceptedTypes.includes(draggingItem.type) : true;

        if (!isTypeAccepted) {
            return { isValid: false };
        }

        const targetNode = dragDropOptions.node();
        const selfDragType = dragDropOptions.dragType();

        // 2. Self-drop prevention
        const isSelfDrop =
            draggingItem.type === selfDragType &&
            String(draggingItem.payload.id) === String(targetNode.id);

        if (isSelfDrop) {
            return { isValid: false };
        }

        // 3. Custom external validation (Domain rules)
        if (!dragDropOptions.isValidDrop(draggingItem, targetNode)) {
            return { isValid: false };
        }

        return { isValid: true };
    });

    /**
     * Accessor function that determines if the node managed by this hook is the current source of a drag operation.
     *
     * @returns {boolean} True if this specific node is currently being dragged.
     */
    const isDraggingSource = () => {
        const activeDragItem = currentDragItem();
        const selfDragType = dragDropOptions.dragType();
        return (
            activeDragItem?.type === selfDragType &&
            String(activeDragItem?.payload?.id) === String(dragDropOptions.node().id)
        );
    };

    /**
     * Accessor function that determines if a drop operation on this node is currently disallowed.
     * Returns true only if there is an active drag and its validation status is false.
     *
     * @returns {boolean} True if the current drop target is invalid.
     */
    const isDropInvalid = () => {
        const activeDragItem = currentDragItem();
        return activeDragItem && !validationStatus().isValid;
    };

    /**
     * Event handler for the 'dragstart' event.
     * Initializes the global drag-and-drop state with the node's data.
     *
     * @param {DragEvent} event - The native browser drag event.
     */
    const handleDragStart = (event: DragEvent) => {
        const selfDragType = dragDropOptions.dragType();
        if (!dragDropOptions.isEnabled() || dragDropOptions.isEditing() || !selfDragType) {
            return;
        }

        event.stopPropagation();
        if (event.dataTransfer) {
            const node = dragDropOptions.node();
            /**
             * Minimal interface for data properties we expect to find in tree nodes.
             */
            interface TreeData {
                path?: string;
                parent_id?: number | null;
            }
            const nodeData = node.data as TreeData;

            const currentDragData: DragItem =
                selfDragType === 'TAG'
                    ? {
                          type: 'TAG',
                          payload: {
                              id: String(node.id),
                              name: node.label,
                              parent_id: nodeData?.parent_id
                          }
                      }
                    : {
                          type: 'ASSET',
                          // This case is rare for tree, but we follow the union
                          payload: {
                              id: String(node.id),
                              ids: [String(node.id)],
                              filename: node.label,
                              path: nodeData?.path || ''
                          }
                      };

            setDragItem(currentDragData);
            event.dataTransfer.effectAllowed = 'all';
            event.dataTransfer.setData('application/json', JSON.stringify(currentDragData));
        }
    };

    /**
     * Event handler for the 'dragend' event.
     * Clears the global drag-and-drop state and visual indicators.
     */
    const handleDragEnd = () => {
        setDragItem(null);
        setDropPosition(null);
    };

    /**
     * Event handler for the 'dragover' event.
     * Calculates the exact drop position ('before', 'after', or 'inside') based on mouse coordinates.
     * Also updates the native 'dropEffect' based on whether it's an internal move or external copy.
     *
     * @param {DragEvent} event - The native browser drag event.
     */
    const handleDragOver = (event: DragEvent) => {
        event.preventDefault();

        if (!dragDropOptions.isEnabled() || !validationStatus().isValid) {
            if (event.dataTransfer) {
                event.dataTransfer.dropEffect = 'none';
            }
            setDropPosition(null);
            return;
        }

        const boundingClientRect = (event.currentTarget as HTMLElement).getBoundingClientRect();
        const mouseYPosition = event.clientY - boundingClientRect.top;
        const totalElementHeight = boundingClientRect.height;
        const edgeThresholdDistance = totalElementHeight * 0.35;

        let calculatedDropPosition: TreeDropPosition = 'inside';

        const draggingItem = currentDragItem();
        const selfDragType = dragDropOptions.dragType();
        const isSiblingDragMatch = draggingItem?.type === selfDragType;

        // Position indicators ('before'/'after') only for sibling moves
        if (isSiblingDragMatch) {
            if (mouseYPosition < edgeThresholdDistance) {
                calculatedDropPosition = 'before';
            } else if (mouseYPosition > totalElementHeight - edgeThresholdDistance) {
                calculatedDropPosition = 'after';
            }
        }

        setDropPosition(calculatedDropPosition);
        if (event.dataTransfer) {
            event.dataTransfer.dropEffect = isSiblingDragMatch ? 'move' : 'copy';
        }
        event.stopPropagation();
    };

    /**
     * Event handler for the 'dragleave' event.
     * Clears the visual drop indicators as the cursor moves away from the node.
     */
    const handleDragLeave = () => {
        setDropPosition(null);
    };

    /**
     * Event handler for the 'drop' event.
     * Resolves the dropped data and invokes the onDrop callback.
     *
     * @param {DragEvent} event - The native browser drop event.
     */
    const handleDrop = async (event: DragEvent) => {
        event.preventDefault();
        event.stopPropagation();

        const finalDropPosition = dropPosition();
        setDropPosition(null);

        if (!validationStatus().isValid) {
            return;
        }

        try {
            let droppedItem: DragItem | null = null;
            const rawJsonData = event.dataTransfer?.getData('application/json');
            if (rawJsonData) {
                try {
                    droppedItem = JSON.parse(rawJsonData);
                } catch {
                    // ignore JSON parse error
                }
            }
            if (!droppedItem) {
                droppedItem = currentDragItem();
            }

            if (droppedItem && dragDropOptions.onDrop) {
                dragDropOptions.onDrop(
                    droppedItem,
                    dragDropOptions.node().id,
                    finalDropPosition || 'inside'
                );
            }
        } catch (error) {
            console.error('Hierarchical drop operation failed:', error);
        } finally {
            setDragItem(null);
        }
    };

    return {
        dropPosition,
        isDraggingSource,
        isDropInvalid,
        handleDragStart,
        handleDragEnd,
        handleDragOver,
        handleDragLeave,
        handleDrop
    };
};
