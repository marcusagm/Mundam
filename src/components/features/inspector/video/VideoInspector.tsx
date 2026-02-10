import { Component, Show } from 'solid-js';
import { type ImageItem } from '../../../../types';
import { Accordion, VideoPlayer as UIVideoPlayer, Loader } from '../../../ui';
import { InspectorTags } from '../base/InspectorTags';
import { CommonMetadata } from '../base/CommonMetadata';
import { useVideoSource } from '../../../../core/hooks/useVideoSource';
import './VideoInspector.css';

interface VideoInspectorProps {
    item: ImageItem;
}

/**
 * Video inspector with HLS streaming support.
 * Uses probe to detect if video needs transcoding and uses HLS for non-native formats.
 */
export const VideoInspector: Component<VideoInspectorProps> = props => {
    // Use consolidated video source hook (default quality is 'standard')
    const { videoUrl, probeResult } = useVideoSource(() => props.item.path);

    return (
        <div class="inspector-content">
            <div class="inspector-preview video-preview">
                <Show when={probeResult.loading}>
                    <div class="inspector-video-loading">
                        <Loader size="md" />
                    </div>
                </Show>

                <Show when={!probeResult.loading && videoUrl()}>
                    <UIVideoPlayer
                        src={videoUrl()}
                        variant="compact"
                        class="inspector-video-player-ui"
                    />
                </Show>
            </div>

            <Accordion>
                <CommonMetadata item={props.item} />
                <InspectorTags itemId={props.item.id} />
            </Accordion>
        </div>
    );
};
