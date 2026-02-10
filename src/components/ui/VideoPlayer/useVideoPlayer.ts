import {
    createSignal,
    createEffect,
    on,
    onMount,
    onCleanup,
    createUniqueId,
    untrack
} from 'solid-js';
import { videoState, videoActions } from '../../../core/store/videoStore';
import { audioActions } from '../../../core/store/audioStore';
import { isHlsUrl } from '../../../lib/stream-utils';
import { HlsPlayerManager } from '../../../lib/hls-player';
import { VideoPlayerProps } from './types';

export function useVideoPlayer(props: VideoPlayerProps) {
    const [videoElement, setVideoElement] = createSignal<HTMLVideoElement | undefined>(undefined);
    const [containerElement, setContainerElement] = createSignal<HTMLDivElement | undefined>(undefined);
    const playerId = createUniqueId();

    const [isPlaying, setIsPlaying] = createSignal(false);
    const [currentTime, setCurrentTime] = createSignal(0);
    const [duration, setDuration] = createSignal(0);
    const [isLoading, setIsLoading] = createSignal(true);
    const [isFullscreen, setIsFullscreen] = createSignal(false);
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
    let hlsManager: HlsPlayerManager | null = null;

    const getStreamMode = () => {
        if (props.src.includes('/hls-live/') || props.src.includes('mode=linear')) return 'LIVE';
        if (isHlsUrl(props.src)) return 'HLS';
        return 'NATIVE';
    };

    // Check if this is a transcoding URL
    const needsTranscode = () =>
        props.src.includes('video-stream://') ||
        props.src.includes('audio-stream://') ||
        isHlsUrl(props.src);

    // Reset state when src changes
    createEffect(
        on(
            () => props.src,
            () => {
                const video = videoElement();
                // Clear any pending retry
                if (retryTimeout) {
                    clearTimeout(retryTimeout);
                    retryTimeout = undefined;
                }
                // Reset playback state
                setError(null);
                setIsLoading(true);
                setIsTranscoding(false);
                setRetryCount(0);
                setCurrentTime(0);
                setDuration(0);
                setBuffered(0);
                setIsPlaying(false);
                setPreviewTime(null);

                // Apply persisted volume to video element
                if (video) {
                    video.volume = videoState.volume();
                    video.muted = videoState.isMuted();
                    video.playbackRate = videoState.playbackRate();
                }
            }
        )
    );

    // Single Active Player Logic
    createEffect(() => {
        const activeId = videoState.activePlayerId();
        const video = videoElement();
        // Use untrack so this effect only runs when activeId changes
        if (activeId && activeId !== playerId && untrack(() => isPlaying())) {
            // Another player started playing, pause this one
            if (video) {
                video.pause();
                setIsPlaying(false);
            }
        }
    });

    // Handle HLS source attachment
    createEffect(
        on(
            [() => props.src, videoElement],
            ([src, video]) => {
                // Cleanup previous HLS instance
                if (hlsManager) {
                    hlsManager.destroy();
                    hlsManager = null;
                }

                if (!video || !src) return;

                // If it's an HLS URL, use hls.js
                if (isHlsUrl(src)) {
                    // Check native HLS support (Safari)
                    if (video.canPlayType('application/vnd.apple.mpegurl')) {
                        // Safari has native support, just set src
                        video.src = src;
                    } else if (HlsPlayerManager.isSupported()) {
                        // Use hls.js
                        hlsManager = new HlsPlayerManager({ debug: false });
                        hlsManager.attach(video, src);
                    } else {
                        setError('HLS playback not supported in this browser');
                    }
                }
                // For non-HLS sources, the video element handles it via src attribute
            }
        )
    );

    const resetControlsTimeout = () => {
        setShowControls(true);
        clearTimeout(controlsTimeout);
        if (isPlaying()) {
            controlsTimeout = window.setTimeout(() => setShowControls(false), 2500);
        }
    };

    const togglePlay = (e?: MouseEvent) => {
        if (e) e.stopPropagation();
        const video = videoElement();
        if (!video) return;

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
        if (!video) return;

        const b = video.buffered;
        const cur = video.currentTime;
        const d = duration();

        if (!Number.isFinite(d) || d <= 0) {
            setBuffered(0);
            return;
        }

        for (let i = 0; i < b.length; i++) {
            if (b.start(i) <= cur && b.end(i) >= cur) {
                setBuffered(b.end(i));
                return;
            }
        }
        if (b.length > 0) {
            setBuffered(b.end(b.length - 1));
        }
    };

    const handleTimeUpdate = () => {
        const video = videoElement();
        if (!video) return;
        setCurrentTime(video.currentTime);
        updateBuffered();
    };

    const handleLoadedMetadata = () => {
        const video = videoElement();
        if (!video) return;

        setIsTranscoding(false);
        setRetryCount(0);
        if (retryTimeout) {
            clearTimeout(retryTimeout);
            retryTimeout = undefined;
        }

        let d = video.duration;
        if ((!Number.isFinite(d) || Number.isNaN(d)) && props.forcedDuration) {
            d = props.forcedDuration;
        }

        setDuration(d);
        setIsLoading(false);
    };

    const handleSeek = (val: number) => {
        const video = videoElement();
        if (!video) return;
        video.currentTime = val;
        setCurrentTime(val);
    };

    const toggleMute = (e?: MouseEvent) => {
        if (e) e.stopPropagation();
        const video = videoElement();
        if (!video) return;
        const newMuted = !videoState.isMuted();
        videoActions.setIsMuted(newMuted);
        video.muted = newMuted;
    };

    const handleVolumeChange = (val: number) => {
        const video = videoElement();
        if (!video) return;
        const v = val / 100;
        videoActions.setVolume(v);
        video.volume = v;
        if (v > 0) videoActions.setIsMuted(false);
    };

    const cyclePlaybackRate = (e?: MouseEvent) => {
        if (e) e.stopPropagation();
        const video = videoElement();
        if (!video) return;
        const rates = [1, 1.25, 1.5, 2];
        const current = videoState.playbackRate();
        const next = rates[(rates.indexOf(current) + 1) % rates.length];
        videoActions.setPlaybackRate(next);
        video.playbackRate = next;
    };

    const toggleFullscreen = (e?: MouseEvent) => {
        if (e) e.stopPropagation();
        const container = containerElement();
        if (!container) return;

        if (!document.fullscreenElement) {
            if (container.requestFullscreen) {
                container.requestFullscreen();
            } else if ((container as any).webkitRequestFullscreen) {
                (container as any).webkitRequestFullscreen();
            }
        } else {
            if (document.exitFullscreen) {
                document.exitFullscreen();
            } else if ((document as any).webkitExitFullscreen) {
                (document as any).webkitExitFullscreen();
            }
        }
    };

    const skip = (seconds: number) => {
        const video = videoElement();
        if (!video) return;
        video.currentTime = Math.min(Math.max(video.currentTime + seconds, 0), duration());
    };

    const handleError = () => {
        const video = videoElement();
        if (!video) return;

        // If this is a transcoding URL and we haven't retried too many times, auto-retry
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
            const msg = needsTranscode()
                ? 'Transcoding failed or timed out'
                : 'Error loading video format';
            setError(msg);
            props.onError?.(msg);
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

    onMount(() => {
        const fsHandler = () => {
            setIsFullscreen(!!document.fullscreenElement);
        };
        document.addEventListener('fullscreenchange', fsHandler);
        document.addEventListener('webkitfullscreenchange', fsHandler);
        resetControlsTimeout();
    });

    onCleanup(() => {
        clearTimeout(controlsTimeout);
        if (retryTimeout) {
            clearTimeout(retryTimeout);
        }
        if (hlsManager) {
            hlsManager.destroy();
            hlsManager = null;
        }
        const fsHandler = () => {};
        document.removeEventListener('fullscreenchange', fsHandler);
        document.removeEventListener('webkitfullscreenchange', fsHandler);
    });

    return {
        // Refs
        videoElement,
        setVideoElement,
        containerElement,
        setContainerElement,

        // State
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

        // Methods
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
        resetControlsTimeout,
    };
}
