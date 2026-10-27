import { DropStrategy, DragItem, DndActionResult } from '../dnd-core';
import { ErrorCode } from '../../types/actions';
import { libraryActions } from '../../store/library';

/**
 * Strategy: Dropping items onto an Asset target.
 */
export const AssetDropStrategy: DropStrategy = {
    accepts: (item: DragItem) => {
        // Only accept TAGS being dropped on assets
        return item.type === 'TAG';
    },

    onDrop: async (item: DragItem, targetId: number | string): Promise<DndActionResult> => {
        if (item.type === 'TAG') {
            const targetAssetId = String(targetId);
            const tagId = String(item.payload.id);

            // Emit intention to library store
            return await libraryActions.applyTagToTarget(tagId, targetAssetId);
        }
        return {
            success: false,
            error: { code: ErrorCode.VALIDATION_ERROR, message: 'Invalid drag item type' }
        };
    },

    onDragOver: (item: DragItem) => {
        return item.type === 'TAG';
    }
};
