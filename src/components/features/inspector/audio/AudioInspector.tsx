import { Component, createSignal, createEffect, Show } from 'solid-js';
import { type ImageItem } from '../../../../types';
import { Accordion, AudioPlayer, Loader } from '../../../ui';
import { InspectorTags } from '../base/InspectorTags';
import { CommonMetadata } from '../base/CommonMetadata';
import './AudioInspector.css';

interface AudioInspectorProps {
    item: ImageItem;
}

export const AudioInspector: Component<AudioInspectorProps> = props => {
    const assetUrl = () => `audio://localhost/${encodeURIComponent(props.item.path)}`;
    const [loading, setLoading] = createSignal(false);

    // Provide a visual hint when switching items in the inspector
    createEffect(() => {
        props.item.id;
        setLoading(true);
        const timer = setTimeout(() => setLoading(false), 400);
        return () => clearTimeout(timer);
    });

    return (
        <div class="inspector-content">
            <div class="inspector-preview audio-preview">
                <Show when={loading()}>
                    <div class="ui-audio-loader">
                        <Loader size="md" />
                    </div>
                </Show>
                <AudioPlayer
                    src={assetUrl()}
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
