import { Component, createSignal, Show } from 'solid-js';
import { VideoPlayer as UIVideoPlayer } from '../../../../ui';
import { Loader } from '../../../../ui/Loader';
import { type TranscodeQuality } from '../../../../../lib/stream-utils';
import { transcodeState } from '../../../../../core/store/transcodeStore';
import { useVideoSource } from '../../../../../core/hooks/useVideoSource';
import '../renderers.css';

interface VideoPlayerProps {
    /** Full file path (not URL) */
    path: string;
}

/**
 * Video renderer with HLS streaming support for non-native formats.
 * Automatically probes the video to detect if it needs transcoding.
 */
export const VideoPlayer: Component<VideoPlayerProps> = props => {
    const [quality, setQuality] = createSignal<TranscodeQuality>(transcodeState.quality());

    // Use consolidated video source hook
    const { videoUrl, probeResult, probeError } = useVideoSource(() => props.path, quality);

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
