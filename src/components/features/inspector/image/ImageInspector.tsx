import { Component } from 'solid-js';
import { type ImageItem } from '../../../../types';
import { CommonMetadata } from '../base/CommonMetadata';
import { ImageMetadata } from './ImageMetadata.tsx';
import { InspectorTags } from '../base/InspectorTags';
import { AdvancedMetadata } from './AdvancedMetadata.tsx';
import { Accordion } from '../../../ui/Accordion';
import './ImageInspector.css';

interface ImageInspectorProps {
    item: ImageItem;
}

export const ImageInspector: Component<ImageInspectorProps> = props => {
    return (
        <div class="inspector-content">
            <div class="inspector-preview square">
                <img
                    class="preview-image"
                    src={
                        props.item.thumbnail_path
                            ? `thumb://localhost/${encodeURIComponent(props.item.thumbnail_path.split(/[\\/]/).pop() || '')}`
                            : ''
                    }
                    alt={props.item.filename}
                />
            </div>

            <Accordion>
                <CommonMetadata item={props.item} />
                <ImageMetadata item={props.item} />
                <InspectorTags itemId={props.item.id} />
                <AdvancedMetadata item={props.item} />
            </Accordion>
        </div>
    );
};
