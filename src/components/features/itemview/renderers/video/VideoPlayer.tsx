import { Component, createSignal, createEffect, on, createResource, Show } from 'solid-js';
import { VideoPlayer as UIVideoPlayer } from '../../../../ui';
import { Loader } from '../../../../ui/Loader';
import {
    type TranscodeQuality,
    probeVideo,
    getHlsPlaylistUrl,
    isHlsServerAvailable,
    type VideoProbeResult,
    needsLinearTranscoding,
    HLS_SERVER_URL,
    getExtension
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

                console.log(
                    `Checking format for ${path}: ext=${getExtension(path)}, linear=${needsLinearTranscoding(path)}`
                );

                // Determine stream URL strategy
                // Check extension-based linear requirement AND codec-based requirement
                // MJPEG and Flash Video (flv1/swf) often fail standard segmentation, so force Linear
                const isLinear =
                    needsLinearTranscoding(path) ||
                    (probe &&
                        (probe.video_codec === 'mjpeg' ||
                            probe.video_codec === 'flv1' ||
                            probe.video_codec === 'vp6f'));

                if (isLinear) {
                    // Linear HLS (Live transcoding) - for formats that can't be segmented easily
                    console.log(`Using Linear HLS for ${path} (isLinear=true)`);
                    // We construct the URL manually or use a helper if available, but getVideoUrl does this too.
                    // However, getVideoUrl returns video-stream:// for standard transcode.
                    // Let's use the direct HTTP URL for Linear to allow the VideoPlayer to handle it as HLS.
                    const encodedPath = encodeURIComponent(path);
                    setVideoUrl(
                        `${HLS_SERVER_URL}/hls-live/${encodedPath}/index.m3u8?quality=${q}`
                    );
                } else if (probe && !probe.is_native) {
                    // Standard HLS (Segmented) - for robust seeking
                    console.log(`Using Standard HLS for ${path} (codec: ${probe.video_codec})`);
                    setVideoUrl(getHlsPlaylistUrl(path, q));
                } else {
                    // Native format (MP4/MOV) - Direct file access
                    console.log(`Using Direct Native for ${path}`);
                    // Native formats uses video:// protocol
                    setVideoUrl(`video://localhost/${encodeURIComponent(path)}`);
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
