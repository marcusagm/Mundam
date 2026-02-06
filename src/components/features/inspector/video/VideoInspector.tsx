import { Component } from 'solid-js';
import { type ImageItem } from '../../../../types';
import { Accordion } from '../../../ui/Accordion';
import { InspectorTags } from '../base/InspectorTags';
import { CommonMetadata } from '../base/CommonMetadata';
import './VideoInspector.css';

interface VideoInspectorProps {
    item: ImageItem;
}

export const VideoInspector: Component<VideoInspectorProps> = props => {
    const assetUrl = () => `video://localhost/${encodeURIComponent(props.item.path)}`;

    return (
        <div class="inspector-content">
            <div class="inspector-preview video-preview">
                <video controls class="inspector-video-player" src={assetUrl()}>
                    Your browser does not support the video element.
                </video>
            </div>

            <Accordion>
                <CommonMetadata item={props.item} />
                <InspectorTags itemId={props.item.id} />
            </Accordion>
        </div>
    );
};
