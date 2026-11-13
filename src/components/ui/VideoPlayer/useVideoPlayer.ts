import { createSignal, createEffect, on, onCleanup, createUniqueId, untrack } from 'solid-js';
import { videoState, videoActions } from '../../../core/store/videoStore';
import { audioActions } from '../../../core/store/audioStore';
import { isHlsUrl } from '../../../lib/stream-utils';
import { VideoPlayerProps } from './types';
import { useFullscreen } from './useFullscreen';
import { usePlayerVolume } from './usePlayerVolume';
import { useHlsAttachment } from './useHlsAttachment';

/**
 * Custom hook to orchestrate the video player state, controls, and lifecycle.
 * Acts as the centralized state unit composed alongside `useHlsAttachment`,
 * `usePlayerVolume`, and `useFullscreen`.
 *
 * @param props - Configuration properties passed to the video player component
 * @returns State signals and action handlers needed by the video interface
 */
export function useVideoPlayer(props: VideoPlayerProps) {
    const [videoElement, setVideoElement] = createSignal<HTMLVideoElement | undefined>(undefined);
    const [containerElement, setContainerElement] = createSignal<HTMLDivElement | undefined>(
        undefined
    );
    const playerId = createUniqueId();

    const [isPlaying, setIsPlaying] = createSignal(false);
    const [currentTime, setCurrentTime] = createSignal(0);
    const [duration, setDuration] = createSignal(0);
    const [isLoading, setIsLoading] = createSignal(true);
    const [showControls, setShowControls] = createSignal(true);
    const [error, setError] = createSignal<string | null>(null);
    const [buffered, setBuffered] = createSignal(0);
    const [previewTime, setPreviewTime] = createSignal<number | null>(null);
    const [previewPos, setPreviewPos] = createSignal(0);
    const [lastAction, setLastAction] = createSignal<'play' | 'pause' | null>(null);
    const [isTranscoding, setIsTranscoding] = createSignal(false);
    const [retryCount, setRetryCount] = createSignal(0);

    let controlsTimeout: number;
    let retryTimeout: number | undefined;

    const { isFullscreen, toggleFullscreen } = useFullscreen();
    const { toggleMute, handleVolumeChange } = usePlayerVolume(videoElement);

    useHlsAttachment({
        sourceUrlAccessor: () => props.src,
        videoElementAccessor: videoElement,
        setErrorState: setError
    });

    const getStreamMode = () => {
        if (props.src.includes('/hls-live/') || props.src.includes('mode=linear')) {
            return 'LIVE';
        }
        if (isHlsUrl(props.src)) {
            return 'HLS';
        }
        return 'NATIVE';
    };

    const needsTranscode = () =>
        props.src.includes('video-stream://') ||
        props.src.includes('audio-stream://') ||
        isHlsUrl(props.src);

    createEffect(
        on(
            () => props.src,
            () => {
                const video = videoElement();
                if (retryTimeout) {
                    clearTimeout(retryTimeout);
                    retryTimeout = undefined;
                }
                setError(null);
                setIsLoading(true);
                setIsTranscoding(false);
                setRetryCount(0);
                setCurrentTime(0);
                setDuration(0);
                setBuffered(0);
                setIsPlaying(false);
                setPreviewTime(null);

                if (video) {
                    video.volume = videoState.volume();
                    video.muted = videoState.isMuted();
                    video.playbackRate = videoState.playbackRate();
                }
            }
        )
    );

    createEffect(() => {
        const activeId = videoState.activePlayerId();
        const video = videoElement();
        if (activeId && activeId !== playerId && untrack(() => isPlaying())) {
            if (video) {
                video.pause();
                setIsPlaying(false);
            }
        }
    });

    const resetControlsTimeout = () => {
        setShowControls(true);
        clearTimeout(controlsTimeout);
        if (isPlaying()) {
            controlsTimeout = window.setTimeout(() => setShowControls(false), 2500);
        }
    };

    const togglePlay = (event?: MouseEvent) => {
        if (event) {
            event.stopPropagation();
        }

        const video = videoElement();
        if (!video) {
            return;
        }

        if (video.paused) {
            video.play();
            setLastAction('play');
        } else {
            video.pause();
            setLastAction('pause');
        }

        setTimeout(() => setLastAction(null), 600);
        resetControlsTimeout();
    };

    const updateBuffered = () => {
        const video = videoElement();
        if (!video) {
            return;
        }

        const bufferedTimeRanges = video.buffered;
        const currentVideoTime = video.currentTime;
        const videoDuration = duration();

        if (!Number.isFinite(videoDuration) || videoDuration <= 0) {
            setBuffered(0);
            return;
        }

        for (let index = 0; index < bufferedTimeRanges.length; index++) {
            if (
                bufferedTimeRanges.start(index) <= currentVideoTime &&
                bufferedTimeRanges.end(index) >= currentVideoTime
            ) {
                setBuffered(bufferedTimeRanges.end(index));
                return;
            }
        }
        if (bufferedTimeRanges.length > 0) {
            setBuffered(bufferedTimeRanges.end(bufferedTimeRanges.length - 1));
        }
    };

    const handleTimeUpdate = () => {
        const video = videoElement();
        if (!video) {
            return;
        }
        setCurrentTime(video.currentTime);
        updateBuffered();
    };

    const handleLoadedMetadata = () => {
        const video = videoElement();
        if (!video) {
            return;
        }

        setIsTranscoding(false);
        setRetryCount(0);
        if (retryTimeout) {
            clearTimeout(retryTimeout);
            retryTimeout = undefined;
        }

        let metadataDuration = video.duration;
        if (
            (!Number.isFinite(metadataDuration) || Number.isNaN(metadataDuration)) &&
            props.forcedDuration
        ) {
            metadataDuration = props.forcedDuration;
        }

        setDuration(metadataDuration);
        setIsLoading(false);
    };

    const handleSeek = (seekValue: number) => {
        const video = videoElement();
        if (!video) {
            return;
        }
        video.currentTime = seekValue;
        setCurrentTime(seekValue);
    };

    const cyclePlaybackRate = (event?: MouseEvent) => {
        if (event) {
            event.stopPropagation();
        }
        const video = videoElement();
        if (!video) {
            return;
        }

        const playbackRates = [1, 1.25, 1.5, 2];
        const currentRate = videoState.playbackRate();
        const nextRate =
            playbackRates[(playbackRates.indexOf(currentRate) + 1) % playbackRates.length];
        videoActions.setPlaybackRate(nextRate);
        video.playbackRate = nextRate;
    };

    const skip = (seconds: number) => {
        const video = videoElement();
        if (!video) {
            return;
        }
        video.currentTime = Math.min(Math.max(video.currentTime + seconds, 0), duration());
    };

    const handleError = () => {
        const video = videoElement();
        if (!video) {
            return;
        }

        // hls.js owns MediaSource recovery. load() would detach its active source.
        if (isHlsUrl(props.src)) {
            return;
        }

        if (needsTranscode() && retryCount() < 20) {
            setIsTranscoding(true);
            setRetryCount(prev => prev + 1);
            retryTimeout = window.setTimeout(() => {
                if (video) {
                    video.load();
                }
            }, 3000);
        } else {
            setIsTranscoding(false);
            const errorMessage = needsTranscode()
                ? 'Transcoding failed or timed out'
                : 'Error loading video format';
            setError(errorMessage);
            props.onError?.(errorMessage);
        }
    };

    const handlePlayParams = () => {
        setIsPlaying(true);
        videoActions.setActivePlayer(playerId);
        audioActions.setActivePlayer('video-player');
        resetControlsTimeout();
    };

    const handlePause = () => {
        setIsPlaying(false);
    };

    onCleanup(() => {
        clearTimeout(controlsTimeout);
        if (retryTimeout) {
            clearTimeout(retryTimeout);
        }
    });

    return {
        videoElement,
        setVideoElement,
        containerElement,
        setContainerElement,

        isPlaying,
        currentTime,
        duration,
        isLoading,
        setIsLoading,
        isFullscreen,
        showControls,
        error,
        setError,
        buffered,
        previewTime,
        setPreviewTime,
        previewPos,
        setPreviewPos,
        lastAction,
        isTranscoding,
        setIsTranscoding,
        retryCount,
        setRetryCount,

        getStreamMode,
        needsTranscode,
        togglePlay,
        updateBuffered,
        handleTimeUpdate,
        handleLoadedMetadata,
        handleSeek,
        toggleMute,
        handleVolumeChange,
        cyclePlaybackRate,
        toggleFullscreen,
        skip,
        handleError,
        handlePlayParams,
        handlePause,
        resetControlsTimeout
    };
}
