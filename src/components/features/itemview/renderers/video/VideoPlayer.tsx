import { Component, createSignal, createEffect, on } from 'solid-js';
import { VideoPlayer as UIVideoPlayer } from '../../../../ui';
import { type TranscodeQuality, getVideoUrl } from '../../../../../lib/stream-utils';
import { transcodeState } from '../../../../../core/store/transcodeStore';
import '../renderers.css';

interface VideoPlayerProps {
    /** Full file path (not URL) */
    path: string;
}

/**
 * Video renderer with quality selector support for transcoded videos.
 * Automatically detects if transcoding is needed and provides quality options.
 * Uses the default quality from settings as initial value.
 */
export const VideoPlayer: Component<VideoPlayerProps> = props => {
    // Initialize with default quality from settings
    const [quality, setQuality] = createSignal<TranscodeQuality>(transcodeState.quality());
    const [videoUrl, setVideoUrl] = createSignal('');

    // Update URL when path or quality changes
    createEffect(
        on(
            () => [props.path, quality()] as const,
            ([path, q]) => {
                setVideoUrl(getVideoUrl(path, q));
            }
        )
    );

    const handleQualityChange = (newQuality: TranscodeQuality) => {
        setQuality(newQuality);
    };

    return (
        <div class="video-player-container">
            <UIVideoPlayer
                src={videoUrl()}
                variant="full"
                autoPlay
                class="video-renderer-player"
                quality={quality()}
                onQualityChange={handleQualityChange}
            />
        </div>
    );
};
