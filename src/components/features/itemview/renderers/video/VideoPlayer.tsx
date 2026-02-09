import { Component, createSignal, createEffect, on, createResource, Show } from 'solid-js';
import { VideoPlayer as UIVideoPlayer } from '../../../../ui';
import { Loader } from '../../../../ui/Loader';
import {
    type TranscodeQuality,
    getVideoUrl,
    probeVideo,
    getHlsPlaylistUrl,
    isHlsServerAvailable,
    type VideoProbeResult
} from '../../../../../lib/stream-utils';
import { transcodeState } from '../../../../../core/store/transcodeStore';
import '../renderers.css';

interface VideoPlayerProps {
    /** Full file path (not URL) */
    path: string;
}

/**
 * Video renderer with HLS streaming support for non-native formats.
 * Automatically probes the video to detect if it needs transcoding.
 * - Native formats (MP4/MOV with H.264): Uses direct video:// protocol
 * - Non-native formats (MKV, AVI, etc): Uses HLS streaming for instant playback
 */
export const VideoPlayer: Component<VideoPlayerProps> = props => {
    const [quality, setQuality] = createSignal<TranscodeQuality>(transcodeState.quality());
    const [videoUrl, setVideoUrl] = createSignal('');
    const [probeError, setProbeError] = createSignal<string | null>(null);

    // Probe video when path changes
    const [probeResult] = createResource(
        () => props.path,
        async (path): Promise<VideoProbeResult | null> => {
            if (!path) return null;

            try {
                // First check if HLS server is available
                const serverAvailable = await isHlsServerAvailable();
                if (!serverAvailable) {
                    console.log('HLS server not available, using fallback');
                    setProbeError(null);
                    return null;
                }

                // Probe the video
                const result = await probeVideo(path);
                setProbeError(null);
                return result;
            } catch (e) {
                console.warn('Video probe failed:', e);
                setProbeError(e instanceof Error ? e.message : 'Probe failed');
                return null;
            }
        }
    );

    // Update URL when path, quality, or probe result changes
    createEffect(
        on(
            () => [props.path, quality(), probeResult()] as const,
            ([path, q, probe]) => {
                if (!path) {
                    setVideoUrl('');
                    return;
                }

                // Determine if we should use HLS
                if (probe && !probe.is_native) {
                    // Non-native format: use HLS streaming
                    console.log(`Using HLS for ${path} (codec: ${probe.video_codec})`);
                    setVideoUrl(getHlsPlaylistUrl(path, q));
                } else {
                    // Native format or probe not available: use direct protocol
                    console.log(`Using direct protocol for ${path}`);
                    setVideoUrl(getVideoUrl(path, q));
                }
            }
        )
    );

    const handleQualityChange = (newQuality: TranscodeQuality) => {
        setQuality(newQuality);
    };

    return (
        <div class="video-player-container">
            <Show when={probeResult.loading}>
                <div class="video-player-loading">
                    <Loader size="lg" />
                    <p>Analyzing video...</p>
                </div>
            </Show>

            <Show when={!probeResult.loading && videoUrl()}>
                <UIVideoPlayer
                    src={videoUrl()}
                    variant="full"
                    autoPlay
                    class="video-renderer-player"
                    quality={quality()}
                    onQualityChange={handleQualityChange}
                    // Show quality selector for all (including HLS)
                    showQualitySelector={true}
                />
            </Show>

            <Show when={probeError()}>
                <div class="video-player-error">
                    <p>Failed to analyze video</p>
                    <small>{probeError()}</small>
                </div>
            </Show>
        </div>
    );
};
