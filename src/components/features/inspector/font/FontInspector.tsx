import { Component, Show } from 'solid-js';
import { type ImageItem } from '../../../../types';
import { Accordion, AccordionItem } from '../../../ui/Accordion';
import { InspectorTags } from '../base/InspectorTags';
import { CommonMetadata } from '../base/CommonMetadata';
import { Type } from 'lucide-solid';
import './FontInspector.css';

interface FontInspectorProps {
    item: ImageItem;
}

export const FontInspector: Component<FontInspectorProps> = props => {
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
                        src={`thumb://localhost/${encodeURIComponent(props.item.thumbnail_path?.split(/[\\/]/).pop() || '')}`}
                        alt={props.item.filename}
                    />
                </Show>
                <div class="font-name">{props.item.filename}</div>
            </div>

            <Accordion>
                <CommonMetadata item={props.item} />
                <AccordionItem
                    value="font-details"
                    title="Typography Info"
                    defaultOpen
                    icon={<Type size={14} />}
                >
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
                </AccordionItem>
                <InspectorTags itemId={props.item.id} />
            </Accordion>
        </div>
    );
};
