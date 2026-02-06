import { Component } from 'solid-js';
import { type ImageItem } from '../../../../types';
import { Accordion, AccordionItem } from '../../../ui/Accordion';
import { InspectorTags } from '../base/InspectorTags';
import { CommonMetadata } from '../base/CommonMetadata';
import { Box, Layers } from 'lucide-solid';
import './ModelInspector.css';

interface ModelInspectorProps {
    item: ImageItem;
}

export const ModelInspector: Component<ModelInspectorProps> = props => {
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
                        src={`thumb://localhost/${encodeURIComponent(props.item.thumbnail_path?.split(/[\\/]/).pop() || '')}`}
                        alt={props.item.filename}
                    />
                </Show>
            </div>

            <Accordion>
                <CommonMetadata item={props.item} />
                <AccordionItem
                    value="model-details"
                    title="3D Model Details"
                    defaultOpen
                    icon={<Layers size={14} />}
                >
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
                </AccordionItem>
                <InspectorTags itemId={props.item.id} />
            </Accordion>
        </div>
    );
};

import { Show } from 'solid-js';
