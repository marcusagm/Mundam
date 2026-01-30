import { Component, createMemo, createResource } from "solid-js";
import { Tag as TagIcon } from "lucide-solid";
import { useMetadata, useSelection, useNotification } from "../../../core/hooks";
import { tagService } from "../../../lib/tags";
import { TagInput, TagOption } from "../../ui/TagInput";
import { AccordionItem } from "../../ui/Accordion";
import "./inspector.css";

export const InspectorTags: Component = () => {
    const metadata = useMetadata();
    const selection = useSelection();
    const notification = useNotification();

    const activeId = createMemo(() => {
        if (selection.selectedIds.length === 0) return null;
        return selection.selectedIds[selection.selectedIds.length - 1]; // Last selected
    });

    const [itemTags, { refetch: refetchTags }] = createResource(
        () => ({ id: activeId(), trigger: metadata.tagUpdateVersion }),
        async ({ id }) => {
            if (!id) return [];
            return await tagService.getTagsForImage(id);
        }
    );

    const tagValue = createMemo<TagOption[]>(() => {
        return (itemTags() || []).map(t => ({
            id: t.id,
            label: t.name,
            color: t.color || undefined
        }));
    });

    const allTagsOptions = createMemo<TagOption[]>(() => {
        return metadata.tags.map(t => ({
            id: t.id,
            label: t.name,
            color: t.color || undefined
        }));
    });

    const handleTagsChange = async (newTags: TagOption[]) => {
        const current = itemTags() || [];
        const currentIds = new Set(current.map(t => t.id));
        const newIds = new Set(newTags.map(t => Number(t.id)));
        
        const sel = selection.selectedIds;
        if (sel.length === 0) return;

        try {
            // Added items
            const added = newTags.filter(t => !currentIds.has(Number(t.id)));
            if (added.length > 0) {
                await tagService.addTagsToImagesBatch([...sel], added.map(t => Number(t.id)));
                const tagNames = added.map(t => t.label).join(", ");
                notification.success("Tags Applied", `Added "${tagNames}" to ${sel.length} item(s)`);
            }

            // Removed items
            const removed = current.filter(t => !newIds.has(t.id));
            if (removed.length > 0) {
                for (const t of removed) {
                    for (const itemId of sel) {
                       await tagService.removeTagFromImage(itemId, t.id);
                    }
                }
                const tagNames = removed.map(t => t.name).join(", ");
                notification.success("Tags Removed", `Removed "${tagNames}" from ${sel.length} item(s)`);
            }
            
            metadata.notifyTagUpdate();
            refetchTags();
        } catch (err) {
            console.error(err);
            notification.error("Failed to Update Tags");
        }
    };

    const handleCreateTag = async (name: string) => {
        const sel = selection.selectedIds;
        if (sel.length === 0) return;
        
        try {
            const newTagId = await tagService.createTag(name);
            await metadata.loadTags(); 
            
            await tagService.addTagsToImagesBatch([...sel], [newTagId]);
            notification.success("Tag Created & Applied", `Created "${name}" and applied to ${sel.length} item(s)`);
            
            metadata.notifyTagUpdate();
            refetchTags();
        } catch (err) {
            console.error(err);
            notification.error("Failed to Create Tag");
        }
    };

    return (
        <AccordionItem 
            value="tags" 
            title="Tags" 
            defaultOpen 
            icon={<TagIcon size={14} />}
        >
            <div class="inspector-tags-wrapper">
                <TagInput 
                    value={tagValue()} 
                    onChange={handleTagsChange} 
                    suggestions={allTagsOptions()}
                    onCreate={handleCreateTag}
                    placeholder="Add tags..."
                />
            </div>
        </AccordionItem>
    );
};
