import { Show, Component, onMount } from 'solid-js';
import { type AssetItem } from '../../../../types';
import { Accordion, AccordionItem, AccordionHeader, AccordionContent } from '../../../ui';
import { InspectorTags } from '../base/InspectorTags';
import { CommonMetadata } from '../base/CommonMetadata';
import { Box, Layers } from 'lucide-solid';
import { accordionActions, accordionStoreState } from '../../../../core/store/accordionStore';
import { getAssetUrl } from '../../../../core/utils/assetUrl';
import './ModelInspector.css';

interface ModelInspectorProps {
    item: AssetItem;
}

export const ModelInspector: Component<ModelInspectorProps> = props => {
    onMount(() => {
        accordionActions.initializeAccordion('inspector_model', ['common']);
    });

    return (
        <div class="inspector-content">
            <div class="inspector-preview model-preview">
                <Show
                    when={props.item.thumbnail_path}
                    fallback={
                        <div class="model-icon-wrapper">
                            <Box size={48} />
                        </div>
                    }
                >
                    <img
                        class="preview-image"
                        src={getAssetUrl(props.item.id, 'thumb')}
                        alt={props.item.filename}
                    />
                </Show>
            </div>

            <Accordion
                type="multiple"
                value={accordionStoreState['inspector_model'] || []}
                onValueChange={val => accordionActions.setExpandedItems('inspector_model', val)}
            >
                <CommonMetadata item={props.item} />
                <AccordionItem value="model-details">
                    <AccordionHeader title="3D Model Details" icon={<Layers size={14} />} />
                    <AccordionContent>
                        <div class="inspector-grid">
                            <div class="inspector-meta-item">
                                <span class="inspector-meta-label">Format</span>
                                <span class="inspector-meta-value">
                                    {props.item.format.toUpperCase()}
                                </span>
                            </div>
                            <div class="inspector-meta-item">
                                <span class="inspector-meta-label">Poly Count</span>
                                <span class="inspector-meta-value">-</span>
                            </div>
                        </div>
                    </AccordionContent>
                </AccordionItem>
                <InspectorTags itemId={props.item.id} />
            </Accordion>
        </div>
    );
};
