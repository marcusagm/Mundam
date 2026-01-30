import { DropStrategy, DragItem } from "../dnd-core";
import { tagService } from "../../../lib/tags";
import { metadataActions, metadataState } from "../../store/metadataStore";
import { toast } from "../../../components/ui/Sonner";

import { selectionState } from "../../store/selectionStore";

// Strategy: Dropping anything ONTO an Image
export const ImageDropStrategy: DropStrategy = {
    accepts: (item: DragItem) => {
        // Only accept TAGS being dropped on images
        return item.type === "TAG";
    },

    onDrop: async (item: DragItem, targetId: number | string) => {
        const targetImageId = Number(targetId);
        
        if (item.type === "TAG") {
            const tagId = Number(item.payload.id);
            
            try {
                // If the target image is in the current selection, apply to ALL selected images
                // Otherwise, just apply to the single target
                let targetIds = [targetImageId];
                if (selectionState.selectedIds.includes(targetImageId)) {
                    targetIds = [...selectionState.selectedIds];
                }
                
                console.log(`Assigning tag ${tagId} to images [${targetIds.join(', ')}]`);

                await tagService.addTagsToImagesBatch(targetIds, [tagId]);
                metadataActions.notifyTagUpdate();
                
                const tagName = metadataState.tags.find(t => t.id === tagId)?.name || "Tag";
                toast.success("Tag Applied", { 
                    description: `Added "${tagName}" to ${targetIds.length} item(s)`
                });
            } catch (err) {
                console.error("Failed to assign tag to image:", err);
                toast.error("Failed to Apply Tag");
            }
        }
    },
    
    onDragOver: (item: DragItem) => {
        return item.type === "TAG";
    }
};
