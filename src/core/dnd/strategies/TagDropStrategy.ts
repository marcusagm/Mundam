import { DropStrategy, DragItem } from "../dnd-core";
import { tagService } from "../../../lib/tags";
import { metadataActions, metadataState } from "../../store/metadataStore";

// Strategy: Dropping anything ONTO a Tag
export const TagDropStrategy: DropStrategy = {
    accepts: (item: DragItem) => {
        return item.type === "IMAGE" || item.type === "TAG";
    },

    onDrop: async (item: DragItem, targetId: number | string, position: "before" | "inside" | "after" = "inside") => {
        let targetTagId: number | null = Number(targetId);
        
        // Handle "root" target (from placeholder)
        if (targetId === "root" || isNaN(targetTagId)) {
            targetTagId = null; 
            // If target is root, position usually "inside" (dropping into root zone)
            if (position !== "inside") {
                // Determine if we should treat before/after root item as root?
                // For now, map to null parent.
                 targetTagId = null;
            }
        }

        // Case 1: Image dropped on Tag (Assignment)
        if (item.type === "IMAGE") {
             // ... existing image logic ...
            const imageIds = item.payload.ids as number[];
            if (targetTagId === null) {
                console.warn("Cannot assign images to root tag container");
                return;
            }
            console.log(`Assigning images [${imageIds}] to tag ${targetTagId}`);
            try {
                await tagService.addTagsToImagesBatch(imageIds, [targetTagId]);
                metadataActions.notifyTagUpdate();
            } catch (err) {
                console.error("Failed to assign tag:", err);
            }
        }

        // Case 2: Tag dropped on Tag (Nesting / Reordering)
        if (item.type === "TAG") {
            const draggedTagId = Number(item.payload.id);
            // Self check already done in UI but good to have
            // if (draggedTagId === targetTagId) return; // Only if inside

            console.log(`Moving tag ${draggedTagId} relative to ${targetId} (${position})`);
            
            try {
                let newParentId: number | null = null;
                
                if (position === "inside") {
                    newParentId = targetTagId; // Nesting
                } else {
                    if (targetTagId !== null) {
                        const targetTag = metadataState.tags.find((t: any) => t.id === targetTagId);
                        newParentId = targetTag ? targetTag.parent_id : null;
                    } else {
                        // Root
                        newParentId = null;
                    }
                }
                
                // Perform the Move + Reorder
                // 1. Update Parent ID first (this conceptually moves it)
                // 2. Then recalculate orders for the destination siblings
                
                // Get fresh state after parent update? Or just calculate now using current state?
                // Using current state is risky if we don't account for the move.
                // Better approach: Construct the desired order list in memory.
                
                const allTags = metadataState.tags;
                
                // Filter siblings of the destination parent, excluding the dragged tag (it's coming here)
                const siblings = allTags
                    .filter((t: any) => t.parent_id === newParentId && t.id !== draggedTagId)
                    .sort((a: any, b: any) => (a.order_index - b.order_index) || a.name.localeCompare(b.name));
                
                // Insert dragged tag into siblings array
                let insertIndex = siblings.length; // Default append (inside)
                
                if (position !== "inside") {
                    const targetIndex = siblings.findIndex((t: any) => t.id === targetTagId);
                    if (targetIndex !== -1) {
                         insertIndex = position === "before" ? targetIndex : targetIndex + 1;
                    }
                }
                
                // Construct new list
                // We represent the dragged item with just its ID for now, or fetch it
                const draggedTag = allTags.find((t: any) => t.id === draggedTagId);
                if (!draggedTag) return; // Should not happen
                
                siblings.splice(insertIndex, 0, draggedTag);
                
                // Update all affected items
                // We'll update order_index for everyone to enforce spacing (0, 100, 200...)
                const updates = siblings.map((t: any, index: number) => {
                    const newOrder = index * 100;
                    // Optimisation: only update if changed?
                    // But for the dragged item, we MUST update parent_id too.
                    const isDragged = t.id === draggedTagId;
                    
                    if (isDragged) {
                        return tagService.updateTag(t.id, undefined, undefined, newParentId === null ? 0 : newParentId, newOrder);
                    } else if (t.order_index !== newOrder) {
                        // Only update order
                         return tagService.updateTag(t.id, undefined, undefined, t.parent_id === null ? 0 : t.parent_id, newOrder);
                    }
                    return Promise.resolve();
                });
                
                await Promise.all(updates);
                metadataActions.loadTags();
            } catch (err) {
                console.error("Failed to move tag:", err);
            }
        }
    },
    
    onDragOver: (_item: DragItem) => {
         return true;
    }
};
