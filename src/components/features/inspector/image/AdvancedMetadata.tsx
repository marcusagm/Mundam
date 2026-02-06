import { Component, createResource, Show, For } from 'solid-js';
import { type ImageItem } from '../../../../types';
import { AccordionItem } from '../../../ui/Accordion';
import { List, Loader2 } from 'lucide-solid';
import { tagService } from '../../../../lib/tags';
import './AdvancedMetadata.css';

interface AdvancedMetadataProps {
    item: ImageItem;
}

const fetchExif = async (path: string) => {
    try {
        return await tagService.getImageExif(path);
    } catch (e) {
        console.error('Failed to load EXIF:', e);
        return {};
    }
};

export const AdvancedMetadata: Component<AdvancedMetadataProps> = props => {
    const [exif] = createResource(() => props.item.path, fetchExif);

    return (
        <AccordionItem value="advanced" title="Advanced Data" icon={<List size={14} />}>
            <Show
                when={!exif.loading}
                fallback={
                    <div class="inspector-loading-spinner">
                        <Loader2 class="animate-spin" size={20} />
                    </div>
                }
            >
                <div class="inspector-field-group">
                    <Show
                        when={Object.keys(exif() || {}).length > 0}
                        fallback={<div class="inspector-no-data">No EXIF data found.</div>}
                    >
                        <div class="inspector-exif-grid">
                            <For each={Object.entries(exif() || {})}>
                                {([key, value]) => (
                                    <div class="inspector-meta-item">
                                        <span class="inspector-meta-label">{key}</span>
                                        <span class="inspector-meta-value exif-value">
                                            {String(value)}
                                        </span>
                                    </div>
                                )}
                            </For>
                        </div>
                    </Show>
                </div>
            </Show>
        </AccordionItem>
    );
};
