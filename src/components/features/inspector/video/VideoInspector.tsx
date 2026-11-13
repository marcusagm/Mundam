import { Component, Show, onMount } from 'solid-js';
import { type AssetItem } from '../../../../types';
import { Accordion, VideoPlayer as UIVideoPlayer, Loader } from '../../../ui';
import { InspectorTags } from '../base/InspectorTags';
import { CommonMetadata } from '../base/CommonMetadata';
import { useVideoSource } from '../../../../core/hooks/useVideoSource';
import { accordionActions, accordionStoreState } from '../../../../core/store/accordionStore';
import './VideoInspector.css';

interface VideoInspectorProps {
    item: AssetItem;
}

/**
 * Video inspector with HLS streaming support.
 * Uses probe to detect if video needs transcoding and uses HLS for non-native formats.
 */
export const VideoInspector: Component<VideoInspectorProps> = props => {
    // Use consolidated video source hook (default quality is 'standard')
    const { videoUrl, probeResult } = useVideoSource(
        () => props.item.id.toString(),
        () => props.item.path
    );

    onMount(() => {
        accordionActions.initializeAccordion('inspector_video', ['common']);
    });

    return (
        <div class="inspector-content">
            <div class="inspector-preview video-preview">
                <Show when={!videoUrl() && probeResult.loading}>
                    <div class="inspector-video-loading">
                        <Loader size="md" />
                    </div>
                </Show>

                <Show when={videoUrl()}>
                    <UIVideoPlayer
                        src={videoUrl()}
                        variant="compact"
                        class="inspector-video-player-ui"
                    />
                </Show>
            </div>

            <Accordion
                type="multiple"
                value={accordionStoreState['inspector_video'] || []}
                onValueChange={val => accordionActions.setExpandedItems('inspector_video', val)}
            >
                <CommonMetadata item={props.item} />
                <InspectorTags itemId={props.item.id} />
            </Accordion>
        </div>
    );
};
