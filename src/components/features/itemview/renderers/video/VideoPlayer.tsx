import { VideoPlayer as UIVideoPlayer } from '../../../../ui';
import { Component, createEffect, createSignal, Show } from 'solid-js';
import { Loader } from '../../../../ui/Loader';
import { type TranscodeQuality } from '../../../../../lib/stream-utils';
import { transcodeState } from '../../../../../core/store/transcodeStore';
import { useVideoSource } from '../../../../../core/hooks/useVideoSource';
import '../renderers.css';

interface VideoPlayerProps {
    /** Asset ID */
    assetId: string;
    /** Full file path (not URL) */
    path: string;
    /** Called once the source is resolved (or failed to resolve) */
    onReady?: () => void;
}

/**
 * Video renderer with HLS streaming support for non-native formats.
 * Automatically probes the video to detect if it needs transcoding.
 */
export const VideoPlayer: Component<VideoPlayerProps> = props => {
    const [quality, setQuality] = createSignal<TranscodeQuality>(transcodeState.quality());

    // Use consolidated video source hook
    const { videoUrl, probeResult, probeError } = useVideoSource(
        () => props.assetId,
        () => props.path,
        quality
    );

    createEffect(() => {
        if (!probeResult.loading && (videoUrl() || probeError())) {
            props.onReady?.();
        }
    });

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
                    // Pass probed duration for linear HLS which reports Infinity
                    forcedDuration={probeResult()?.duration_secs}
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
