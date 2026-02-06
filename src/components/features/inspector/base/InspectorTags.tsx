import { Component, createResource } from 'solid-js';
import { Tag as TagIcon } from 'lucide-solid';
import { AccordionItem } from '../../../ui/Accordion';
import { TagInput, TagOption } from '../../../ui/TagInput';
import { tagService } from '../../../../lib/tags';
import './InspectorTags.css';

interface InspectorTagsProps {
    itemId?: number;
    itemIds?: number[];
}

export const InspectorTags: Component<InspectorTagsProps> = props => {
    const [allTags] = createResource(tagService.getAllTags);
    const [itemTags, { refetch }] = createResource(
        () => props.itemId,
        async id => {
            if (typeof id !== 'number') return [];
            return await tagService.getTagsForImage(id);
        }
    );

    const tagOptions = () =>
        (allTags() || []).map(t => ({ id: t.id, label: t.name, color: t.color || undefined }));
    const selectedOptions = () =>
        (itemTags() || []).map(t => ({ id: t.id, label: t.name, color: t.color || undefined }));

    const handleChange = async (newOptions: TagOption[]) => {
        const itemId = props.itemId;
        const itemIds = props.itemIds;
        if (itemId === undefined && (!itemIds || itemIds.length === 0)) return;

        const ids = itemId !== undefined ? [itemId] : itemIds!;
        const currentSelected = selectedOptions();
        const currentIds = new Set(currentSelected.map(o => String(o.id)));
        const newIds = new Set(newOptions.map(o => String(o.id)));

        // Additions
        const toAdd = newOptions.filter(o => !currentIds.has(String(o.id))).map(o => Number(o.id));

        if (toAdd.length > 0) {
            await tagService.addTagsToImagesBatch(ids, toAdd);
        }

        // Removals
        const toRemove = currentSelected
            .filter(o => !newIds.has(String(o.id)))
            .map(o => Number(o.id));

        for (const tagId of toRemove) {
            for (const id of ids) {
                await tagService.removeTagFromImage(id, tagId);
            }
        }

        refetch();
    };

    const handleCreate = async (name: string) => {
        const itemId = props.itemId;
        const itemIds = props.itemIds;
        if (itemId === undefined && (!itemIds || itemIds.length === 0)) return;

        const id = await tagService.createTag(name);
        const ids = itemId !== undefined ? [itemId] : itemIds!;
        await tagService.addTagsToImagesBatch(ids, [id]);
        refetch();
    };

    return (
        <AccordionItem value="tags" title="Tags" icon={<TagIcon size={14} />}>
            <div class="inspector-tags-wrapper">
                <TagInput
                    value={selectedOptions()}
                    suggestions={tagOptions()}
                    onChange={handleChange}
                    onCreate={handleCreate}
                    placeholder="Add tags..."
                />
            </div>
        </AccordionItem>
    );
};
