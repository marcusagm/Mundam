import { Component, For } from 'solid-js';
import { type ImageItem } from '../../../../types';
import { InspectorTags } from '../base/InspectorTags';
import { Accordion, AccordionItem } from '../../../ui/Accordion';
import { Layers } from 'lucide-solid';
import './MultiInspector.css';

interface MultiInspectorProps {
    items: ImageItem[];
}

export const MultiInspector: Component<MultiInspectorProps> = props => {
    if (!props.items || props.items.length === 0) {
        return <div class="inspector-content empty">No selection</div>;
    }

    const previewItems = () => props.items.slice(0, 3).reverse();

    return (
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
                                        item.thumbnail_path
                                            ? `thumb://localhost/${encodeURIComponent(item.thumbnail_path.split(/[\\/]/).pop() || '')}`
                                            : ''
                                    }
                                    class="deck-card-image"
                                />
                            </div>
                        )}
                    </For>
                    <div class="inspector-deck-badge">{props.items.length}</div>
                </div>
            </div>

            <div class="inspector-selection-count">{props.items.length} items selected</div>

            <Accordion>
                <InspectorTags itemIds={props.items.map(i => i.id)} />
                <AccordionItem value="info" title="Batch Actions" icon={<Layers size={14} />}>
                    <div class="inspector-field-group">
                        <p class="batch-hint">
                            Editing tags will apply to all {props.items.length} selected items.
                        </p>
                    </div>
                </AccordionItem>
            </Accordion>
        </div>
    );
};
