import { Component, Show, onMount } from 'solid-js';
import { type AssetItem } from '../../../../types';
import { Accordion, AccordionItem, AccordionHeader, AccordionContent } from '../../../ui';
import { InspectorTags } from '../base/InspectorTags';
import { CommonMetadata } from '../base/CommonMetadata';
import { Type } from 'lucide-solid';
import { accordionActions, accordionStoreState } from '../../../../core/store/accordionStore';
import { getAssetUrl } from '../../../../core/utils/assetUrl';
import './FontInspector.css';

interface FontInspectorProps {
    item: AssetItem;
}

export const FontInspector: Component<FontInspectorProps> = props => {
    onMount(() => {
        accordionActions.initializeAccordion('inspector_font', ['common']);
    });

    return (
        <div class="inspector-content">
            <div class="inspector-preview font-preview">
                <Show
                    when={props.item.thumbnail_path}
                    fallback={
                        <div class="font-sampler">
                            <span class="sampler-text">Ag</span>
                        </div>
                    }
                >
                    <img
                        class="preview-image"
                        src={getAssetUrl(props.item.id, 'thumb')}
                        alt={props.item.filename}
                    />
                </Show>
                <div class="font-name">{props.item.filename}</div>
            </div>

            <Accordion
                type="multiple"
                value={accordionStoreState['inspector_font'] || []}
                onValueChange={val => accordionActions.setExpandedItems('inspector_font', val)}
            >
                <CommonMetadata item={props.item} />
                <AccordionItem value="font-details">
                    <AccordionHeader title="Typography Info" icon={<Type size={14} />} />
                    <AccordionContent>
                        <div class="inspector-grid">
                            <div class="inspector-meta-item">
                                <span class="inspector-meta-label">Format</span>
                                <span class="inspector-meta-value">
                                    {props.item.format.toUpperCase()}
                                </span>
                            </div>
                            <div class="inspector-meta-item">
                                <span class="inspector-meta-label">Extension</span>
                                <span class="inspector-meta-value">
                                    .{props.item.filename.split('.').pop()}
                                </span>
                            </div>
                        </div>
                    </AccordionContent>
                </AccordionItem>
                <InspectorTags itemId={props.item.id} />
            </Accordion>
        </div>
    );
};
