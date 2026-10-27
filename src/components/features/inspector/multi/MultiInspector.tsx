import { Component, For, Show, onMount } from 'solid-js';
import { type AssetItem } from '../../../../types';
import { InspectorTags } from '../base/InspectorTags';
import { Accordion, AccordionItem, AccordionHeader, AccordionContent } from '../../../ui';
import { Layers } from 'lucide-solid';
import { accordionActions, accordionStoreState } from '../../../../core/store/accordionStore';
import { getAssetUrl } from '../../../../core/utils/assetUrl';
import './MultiInspector.css';

interface MultiInspectorProps {
    items: AssetItem[];
}

export const MultiInspector: Component<MultiInspectorProps> = props => {
    const previewItems = () => (props.items || []).slice(0, 3).reverse();
    const selectionCount = () => props.items?.length || 0;

    onMount(() => {
        accordionActions.initializeAccordion('inspector_multi', ['info']);
    });

    return (
        <Show
            when={selectionCount() > 0}
            fallback={<div class="inspector-content empty">No selection</div>}
        >
            <div class="inspector-content">
                <div class="inspector-preview deck-container">
                    <div class="inspector-deck-wrapper">
                        <For each={previewItems()}>
                            {(item, index) => (
                                <div
                                    class="inspector-deck-card"
                                    style={{
                                        top: `${index() * 4}px`,
                                        left: `${index() * 4}px`,
                                        right: `${(2 - index()) * 4}px`,
                                        bottom: `${(2 - index()) * 4}px`,
                                        transform: `rotate(${(index() - 1) * 3}deg)`,
                                        'z-index': index()
                                    }}
                                >
                                    <img
                                        src={
                                            item.thumbnail_path ? getAssetUrl(item.id, 'thumb') : ''
                                        }
                                        class="deck-card-image"
                                    />
                                </div>
                            )}
                        </For>
                        <div class="inspector-deck-badge">{selectionCount()}</div>
                    </div>
                </div>

                <div class="inspector-selection-count">{selectionCount()} items selected</div>

                <Accordion
                    type="multiple"
                    value={accordionStoreState['inspector_multi'] || []}
                    onValueChange={val => accordionActions.setExpandedItems('inspector_multi', val)}
                >
                    <InspectorTags itemIds={props.items.map(i => i.id)} />
                    <AccordionItem value="info">
                        <AccordionHeader title="Batch Actions" icon={<Layers size={14} />} />
                        <AccordionContent>
                            <div class="inspector-field-group">
                                <p class="batch-hint">
                                    Editing tags will apply to all {selectionCount()} selected
                                    items.
                                </p>
                            </div>
                        </AccordionContent>
                    </AccordionItem>
                </Accordion>
            </div>
        </Show>
    );
};
