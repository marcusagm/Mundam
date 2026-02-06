import { Component } from 'solid-js';
import { type ImageItem } from '../../../../types';
import { Accordion } from '../../../ui/Accordion';
import { InspectorTags } from '../base/InspectorTags';
import { CommonMetadata } from '../base/CommonMetadata';
import './AudioInspector.css';

interface AudioInspectorProps {
    item: ImageItem;
}

export const AudioInspector: Component<AudioInspectorProps> = props => {
    const assetUrl = () => `orig://localhost/${encodeURIComponent(props.item.path)}`;

    return (
        <div class="inspector-content">
            <div class="inspector-preview audio-preview">
                <audio controls class="inspector-audio-player" src={assetUrl()}>
                    Your browser does not support the audio element.
                </audio>
            </div>

            <Accordion>
                <CommonMetadata item={props.item} />
                <InspectorTags itemId={props.item.id} />
            </Accordion>
        </div>
    );
};
