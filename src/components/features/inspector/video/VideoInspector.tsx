import { Component, createSignal, createEffect, on, createResource, Show } from 'solid-js';
import { type ImageItem } from '../../../../types';
import { Accordion, VideoPlayer as UIVideoPlayer, Loader } from '../../../ui';
import { InspectorTags } from '../base/InspectorTags';
import { CommonMetadata } from '../base/CommonMetadata';
import {
    getVideoUrl,
    probeVideo,
    getHlsPlaylistUrl,
    isHlsServerAvailable
} from '../../../../lib/stream-utils';
import './VideoInspector.css';

interface VideoInspectorProps {
    item: ImageItem;
}

/**
 * Video inspector with HLS streaming support.
 * Uses probe to detect if video needs transcoding and uses HLS for non-native formats.
 */
export const VideoInspector: Component<VideoInspectorProps> = props => {
    const [videoUrl, setVideoUrl] = createSignal('');

    // Probe video when item changes
    const [probeResult] = createResource(
        () => props.item.path,
        async path => {
            if (!path) return null;

            try {
                const serverAvailable = await isHlsServerAvailable();
                if (!serverAvailable) {
                    console.log('HLS server not available for inspector, using fallback');
                    return null;
                }
                return await probeVideo(path);
            } catch (e) {
                console.warn('Video probe failed in inspector:', e);
                return null;
            }
        }
    );

    // Update URL based on probe result
    createEffect(
        on(
            () => [props.item.path, probeResult()] as const,
            ([path, probe]) => {
                if (!path) {
                    setVideoUrl('');
                    return;
                }

                if (probe && !probe.is_native) {
                    // Non-native format: use HLS
                    console.log(`Inspector: Using HLS for ${path}`);
                    setVideoUrl(getHlsPlaylistUrl(path));
                } else {
                    // Native or fallback: use direct protocol
                    console.log(`Inspector: Using direct protocol for ${path}`);
                    setVideoUrl(getVideoUrl(path));
                }
            }
        )
    );

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
