import { Component } from 'solid-js';
import { type ImageItem } from '../../../../types';
import { Accordion, AudioPlayer } from '../../../ui';
import { InspectorTags } from '../base/InspectorTags';
import { CommonMetadata } from '../base/CommonMetadata';
import { useAudioSource } from '../../../../core/hooks/useAudioSource';
import './AudioInspector.css';

interface AudioInspectorProps {
    item: ImageItem;
}

export const AudioInspector: Component<AudioInspectorProps> = props => {
    const { audioUrl } = useAudioSource(() => props.item.path);

    return (
        <div class="inspector-content">
            <div class="inspector-preview audio-preview">
                <AudioPlayer
                    src={audioUrl()}
                    filePath={props.item.path}
                    variant="compact"
                    class="inspector-audio-player"
                />
            </div>

            <Accordion>
                <CommonMetadata item={props.item} />
                <InspectorTags itemId={props.item.id} />
            </Accordion>
        </div>
    );
};
