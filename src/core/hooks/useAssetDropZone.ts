import { createSignal } from 'solid-js';
import { setDropTargetId, currentDropTargetId, currentDragItem, DragItem } from '../dnd';
import { useDndHandlers } from './useDndHandlers';

/**
 * Specialized hook for managing drop zone behavior on assets (AssetItems).
 * Handles drag counters to prevent flickering and coordinates with the DND handler.
 *
 * @param getAssetId - A reactive accessor for the asset ID. This ensures the hook
 * stays correctly bound to the current item even if the underlying DOM node is
 * recycled in a virtualized list.
 */
export const useAssetDropZone = (getAssetId: () => string) => {
    const [dragCounter, setDragCounter] = createSignal(0);
    const { handleDrop } = useDndHandlers();

    /**
     * Helper to get the current ID from the reactive accessor.
     */
    const getTargetId = () => getAssetId();

    const isDropTarget = () => currentDropTargetId() === getTargetId();

    const onDragEnter = (event: DragEvent) => {
        event.preventDefault();
        const nextCounter = dragCounter() + 1;
        setDragCounter(nextCounter);

        const dragging = currentDragItem();
        const isTag =
            dragging?.type === 'TAG' || event.dataTransfer?.types?.includes('application/json');

        if (isTag) {
            if (event.dataTransfer) {
                const allowed = event.dataTransfer.effectAllowed;
                event.dataTransfer.dropEffect = allowed === 'move' ? 'move' : 'copy';
            }
            if (nextCounter === 1) {
                setDropTargetId(getTargetId());
            }
        }
    };

    const onDragOver = (event: DragEvent) => {
        event.preventDefault();
        const dragging = currentDragItem();
        const isTag =
            dragging?.type === 'TAG' || event.dataTransfer?.types?.includes('application/json');

        if (isTag) {
            if (event.dataTransfer) {
                const allowed = event.dataTransfer.effectAllowed;
                event.dataTransfer.dropEffect = allowed === 'move' ? 'move' : 'copy';
            }
            const currentId = getTargetId();
            if (currentDropTargetId() !== currentId) {
                setDropTargetId(currentId);
            }
        }
    };

    const onDragLeave = () => {
        const nextCounter = Math.max(0, dragCounter() - 1);
        setDragCounter(nextCounter);
        if (nextCounter === 0 && currentDropTargetId() === getTargetId()) {
            setDropTargetId(null);
        }
    };

    const onDrop = async (event: DragEvent) => {
        event.preventDefault();
        event.stopPropagation();
        setDragCounter(0);
        setDropTargetId(null);

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

            // Asset cards specifically only allow TAG drops for tag assignment
            if (droppedItem?.type === 'TAG') {
                await handleDrop(droppedItem, getTargetId(), 'ASSET');
            }
        } catch (error) {
            console.error('Asset drop failed:', error);
        }
    };

    return {
        isDropTarget,
        dragHandlers: {
            onDragEnter,
            onDragOver,
            onDragLeave,
            onDrop
        }
    };
};
