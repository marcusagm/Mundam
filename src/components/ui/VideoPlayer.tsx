import { Component, createSignal, createEffect, on, onMount, onCleanup, Show, For } from 'solid-js';
import { cn } from '../../lib/utils';
import { Button } from './Button';
import { Slider } from './Slider';
import { Tooltip } from './Tooltip';
import { Loader } from './Loader';
import { Popover } from './Popover';
import {
    Play,
    Pause,
    Volume2,
    VolumeX,
    Maximize,
    Minimize,
    SkipBack,
    SkipForward,
    Settings,
    Check
} from 'lucide-solid';
import { videoState, videoActions } from '../../core/store/videoStore';
import { type TranscodeQuality, isHlsUrl } from '../../lib/stream-utils';
import { HlsPlayerManager } from '../../lib/hls-player';
import './video-player.css';

export type QualityOption = {
    id: TranscodeQuality;
    label: string;
};

const QUALITY_OPTIONS: QualityOption[] = [
    { id: 'preview', label: 'Preview' },
    { id: 'standard', label: 'Standard' },
    { id: 'high', label: 'High' }
];

export interface VideoPlayerProps {
    src: string;
    variant?: 'full' | 'compact';
    autoPlay?: boolean;
    title?: string;
    quality?: TranscodeQuality;
    onQualityChange?: (quality: TranscodeQuality) => void;
    onEnded?: () => void;
    onError?: (error: string) => void;
    /** Show quality selector button (default: true for transcoded videos) */
    showQualitySelector?: boolean;
    class?: string;
}

export const VideoPlayer: Component<VideoPlayerProps> = props => {
    let videoRef: HTMLVideoElement | undefined;
    let containerRef: HTMLDivElement | undefined;

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

    // Check if this is a transcoding URL
    const needsTranscode = () =>
        props.src.includes('video-stream://') || props.src.includes('audio-stream://');
    // HLS Manager instance
    let hlsManager: HlsPlayerManager | null = null;

    // Reset state when src changes
    createEffect(
        on(
            () => props.src,
            () => {
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
                if (videoRef) {
                    videoRef.volume = videoState.volume();
                    videoRef.muted = videoState.isMuted();
                    videoRef.playbackRate = videoState.playbackRate();
                }
            }
        )
    );

    // Handle HLS source attachment
    createEffect(
        on(
            () => props.src,
            src => {
                // Cleanup previous HLS instance
                if (hlsManager) {
                    hlsManager.destroy();
                    hlsManager = null;
                }

                if (!videoRef || !src) return;

                // If it's an HLS URL, use hls.js
                if (isHlsUrl(src)) {
                    // Check native HLS support (Safari)
                    if (videoRef.canPlayType('application/vnd.apple.mpegurl')) {
                        // Safari has native support, just set src
                        videoRef.src = src;
                    } else if (HlsPlayerManager.isSupported()) {
                        // Use hls.js
                        hlsManager = new HlsPlayerManager({ debug: false });
                        hlsManager.attach(videoRef, src);
                    } else {
                        setError('HLS playback not supported in this browser');
                    }
                }
                // For non-HLS sources, the video element handles it via src attribute
            }
        )
    );

    const togglePlay = (e?: MouseEvent) => {
        if (e) e.stopPropagation();
        if (!videoRef) return;

        if (videoRef.paused) {
            videoRef.play();
            setLastAction('play');
        } else {
            videoRef.pause();
            setLastAction('pause');
        }

        // Hide indicator after animation
        setTimeout(() => setLastAction(null), 600);
        resetControlsTimeout();
    };

    const resetControlsTimeout = () => {
        setShowControls(true);
        clearTimeout(controlsTimeout);
        if (isPlaying()) {
            controlsTimeout = window.setTimeout(() => setShowControls(false), 2500);
        }
    };

    const formatTime = (seconds: number) => {
        const h = Math.floor(seconds / 3600);
        const m = Math.floor((seconds % 3600) / 60);
        const s = Math.floor(seconds % 60);
        const parts = [m, s].map(v => v.toString().padStart(2, '0'));
        if (h > 0) parts.unshift(h.toString());
        return parts.join(':');
    };

    const handleTimeUpdate = () => {
        if (!videoRef) return;
        setCurrentTime(videoRef.currentTime);
        updateBuffered();
    };

    const updateBuffered = () => {
        if (!videoRef) return;
        const b = videoRef.buffered;
        const cur = videoRef.currentTime;
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

    const handleLoadedMetadata = () => {
        if (!videoRef) return;
        // Video loaded successfully - clear transcoding state
        setIsTranscoding(false);
        setRetryCount(0);
        if (retryTimeout) {
            clearTimeout(retryTimeout);
            retryTimeout = undefined;
        }
        setDuration(videoRef.duration);
        setIsLoading(false);
    };

    const handleSeek = (val: number) => {
        if (!videoRef) return;
        videoRef.currentTime = val;
        setCurrentTime(val);
    };

    const toggleMute = (e?: MouseEvent) => {
        if (e) e.stopPropagation();
        if (!videoRef) return;
        const newMuted = !videoState.isMuted();
        videoActions.setIsMuted(newMuted);
        videoRef.muted = newMuted;
    };

    const handleVolumeChange = (val: number) => {
        if (!videoRef) return;
        const v = val / 100;
        videoActions.setVolume(v);
        videoRef.volume = v;
        if (v > 0) videoActions.setIsMuted(false);
    };

    const cyclePlaybackRate = (e?: MouseEvent) => {
        if (e) e.stopPropagation();
        if (!videoRef) return;
        const rates = [1, 1.25, 1.5, 2];
        const current = videoState.playbackRate();
        const next = rates[(rates.indexOf(current) + 1) % rates.length];
        videoActions.setPlaybackRate(next);
        videoRef.playbackRate = next;
    };

    const toggleFullscreen = (e?: MouseEvent) => {
        if (e) e.stopPropagation();
        if (!containerRef) return;
        if (!document.fullscreenElement) {
            if (containerRef.requestFullscreen) {
                containerRef.requestFullscreen();
            } else if ((containerRef as any).webkitRequestFullscreen) {
                (containerRef as any).webkitRequestFullscreen();
            }
        } else {
            if (document.exitFullscreen) {
                document.exitFullscreen();
            } else if ((document as any).webkitExitFullscreen) {
                (document as any).webkitExitFullscreen();
            }
        }
    };

    const handleSeekMouseMove = (e: MouseEvent) => {
        const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
        const pos = (e.clientX - rect.left) / rect.width;
        const time = pos * duration();
        setPreviewTime(time);
        setPreviewPos(pos * 100);
    };

    const skip = (seconds: number) => {
        if (!videoRef) return;
        videoRef.currentTime = Math.min(Math.max(videoRef.currentTime + seconds, 0), duration());
    };

    const handleKeyDown = (e: KeyboardEvent) => {
        resetControlsTimeout();
        switch (e.code) {
            case 'Space':
            case 'KeyK':
                e.preventDefault();
                togglePlay();
                break;
            case 'ArrowLeft':
            case 'KeyJ':
                e.preventDefault();
                skip(-5);
                break;
            case 'ArrowRight':
            case 'KeyL':
                e.preventDefault();
                skip(5);
                break;
            case 'ArrowUp':
                e.preventDefault();
                handleVolumeChange(Math.min((videoState.volume() + 0.1) * 100, 100));
                break;
            case 'ArrowDown':
                e.preventDefault();
                handleVolumeChange(Math.max((videoState.volume() - 0.1) * 100, 0));
                break;
            case 'KeyF':
                e.preventDefault();
                toggleFullscreen();
                break;
            case 'KeyM':
                e.preventDefault();
                toggleMute();
                break;
        }
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
        // Cleanup HLS manager
        if (hlsManager) {
            hlsManager.destroy();
            hlsManager = null;
        }
        const fsHandler = () => {};
        document.removeEventListener('fullscreenchange', fsHandler);
        document.removeEventListener('webkitfullscreenchange', fsHandler);
    });

    return (
        <div
            ref={containerRef}
            class={cn(
                'ui-video-player',
                props.variant === 'compact' && 'ui-video-player-compact',
                props.class
            )}
            onMouseMove={resetControlsTimeout}
            onMouseEnter={resetControlsTimeout}
            onContextMenu={resetControlsTimeout}
            onKeyDown={handleKeyDown}
            tabindex="0"
        >
            <Show when={isTranscoding()}>
                <div class="ui-video-transcoding">
                    <Loader size="lg" />
                    <p>Transcoding video...</p>
                    <p class="ui-video-transcoding-hint">This may take a while for large files</p>
                </div>
            </Show>

            <Show when={error() && !isTranscoding()}>
                <div class="ui-video-error">
                    <AlertCircle size={48} />
                    <p>{error()}</p>
                    <Button
                        variant="outline"
                        onClick={() => {
                            setError(null);
                            setRetryCount(0);
                        }}
                    >
                        Retry
                    </Button>
                </div>
            </Show>

            <Show when={!error()}>
                <video
                    ref={videoRef}
                    src={props.src}
                    class="ui-video-element"
                    autoplay={props.autoPlay}
                    preload="metadata"
                    onPlay={() => {
                        setIsPlaying(true);
                        resetControlsTimeout();
                    }}
                    onPause={() => {
                        setIsPlaying(false);
                    }}
                    onTimeUpdate={() => {
                        handleTimeUpdate();
                        updateBuffered();
                    }}
                    onProgress={updateBuffered}
                    onSeeking={updateBuffered}
                    onSeeked={updateBuffered}
                    onLoadedMetadata={handleLoadedMetadata}
                    onWaiting={() => setIsLoading(true)}
                    onPlaying={() => setIsLoading(false)}
                    onEnded={props.onEnded}
                    onClick={togglePlay}
                    onDblClick={toggleFullscreen}
                    onError={() => {
                        // If this is a transcoding URL and we haven't retried too many times, auto-retry
                        if (needsTranscode() && retryCount() < 20) {
                            setIsTranscoding(true);
                            setRetryCount(prev => prev + 1);
                            // Auto-retry after 3 seconds
                            retryTimeout = window.setTimeout(() => {
                                if (videoRef) {
                                    videoRef.load();
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
                    }}
                    tabindex="-1"
                />

                <Show when={isLoading()}>
                    <div class="ui-video-loader">
                        <Loader size="lg" />
                    </div>
                </Show>

                <div class="ui-video-center-action">
                    <Show when={lastAction() === 'play'}>
                        <div class="ui-video-center-action-pulse">
                            <Play size={48} fill="currentColor" />
                        </div>
                    </Show>
                    <Show when={lastAction() === 'pause'}>
                        <div class="ui-video-center-action-pulse">
                            <Pause size={48} fill="currentColor" />
                        </div>
                    </Show>
                </div>

                <div
                    class={cn(
                        'ui-video-controls',
                        !isPlaying() && 'ui-video-controls-visible',
                        showControls() && 'ui-video-controls-visible'
                    )}
                >
                    <Show when={props.title && props.variant === 'full'}>
                        <div class="ui-video-top-bar">{props.title}</div>
                    </Show>

                    <div class="ui-video-bottom-controls">
                        {/* Seek Bar Area */}
                        <div
                            class="ui-video-seekbar-container"
                            onMouseMove={handleSeekMouseMove}
                            onMouseLeave={() => setPreviewTime(null)}
                        >
                            <Show when={previewTime() !== null}>
                                <div
                                    class="ui-video-seekbar-preview"
                                    style={{ left: `${previewPos()}%` }}
                                >
                                    {formatTime(previewTime()!)}
                                </div>
                            </Show>
                            <div class="ui-video-seekbar">
                                <div
                                    class="ui-video-seekbar-buffer"
                                    style={{ width: `${(buffered() / duration()) * 100}%` }}
                                />
                                <Slider
                                    min={0}
                                    max={duration()}
                                    step={0.1}
                                    value={currentTime()}
                                    onValueChange={handleSeek}
                                    class="ui-video-seekbar-slider"
                                />
                            </div>
                        </div>

                        <div class="ui-video-controls-row">
                            <div class="ui-video-controls-left ui-video-controls-row">
                                <Tooltip content={isPlaying() ? 'Pause' : 'Play'}>
                                    <Button
                                        variant="ghost"
                                        size="icon-sm"
                                        onClick={e => togglePlay(e)}
                                    >
                                        <Show
                                            when={isPlaying()}
                                            fallback={<Play size={18} fill="currentColor" />}
                                        >
                                            <Pause size={18} fill="currentColor" />
                                        </Show>
                                    </Button>
                                </Tooltip>

                                <Show when={props.variant === 'full'}>
                                    <Tooltip content="Step backward 5s">
                                        <Button
                                            variant="ghost"
                                            size="icon-sm"
                                            onClick={() => skip(-5)}
                                        >
                                            <SkipBack size={18} fill="currentColor" />
                                        </Button>
                                    </Tooltip>
                                    <Tooltip content="Step forward 5s">
                                        <Button
                                            variant="ghost"
                                            size="icon-sm"
                                            onClick={() => skip(5)}
                                        >
                                            <SkipForward size={18} fill="currentColor" />
                                        </Button>
                                    </Tooltip>
                                </Show>

                                <div class="ui-video-volume-group">
                                    <Button
                                        variant="ghost"
                                        size="icon-sm"
                                        onClick={e => toggleMute(e)}
                                    >
                                        <Show
                                            when={videoState.isMuted() || videoState.volume() === 0}
                                            fallback={<Volume2 size={18} />}
                                        >
                                            <VolumeX size={18} />
                                        </Show>
                                    </Button>
                                    <div class="ui-video-volume-slider">
                                        <Slider
                                            min={0}
                                            max={100}
                                            value={
                                                videoState.isMuted() ? 0 : videoState.volume() * 100
                                            }
                                            onValueChange={handleVolumeChange}
                                        />
                                    </div>
                                </div>

                                <div class="ui-video-time">
                                    {formatTime(currentTime())} <span>/</span>{' '}
                                    {formatTime(duration())}
                                </div>
                            </div>

                            <div style={{ flex: 1 }} />

                            <div class="ui-video-controls-right ui-video-controls-row">
                                <Show when={props.variant === 'full'}>
                                    <Tooltip content="Playback Speed">
                                        <Button
                                            variant="ghost"
                                            size="sm"
                                            class="ui-video-speed-btn"
                                            onClick={e => cyclePlaybackRate(e)}
                                        >
                                            {videoState.playbackRate()}x
                                        </Button>
                                    </Tooltip>
                                </Show>

                                {/* Quality Selector - only for transcoded videos when enabled */}
                                <Show
                                    when={
                                        props.showQualitySelector !== false &&
                                        needsTranscode() &&
                                        props.onQualityChange
                                    }
                                >
                                    <Popover
                                        trigger={
                                            <Tooltip content="Quality">
                                                <Button
                                                    variant="ghost"
                                                    size="icon-sm"
                                                    class="ui-video-quality-btn"
                                                >
                                                    <Settings size={18} />
                                                </Button>
                                            </Tooltip>
                                        }
                                        align="end"
                                    >
                                        <div class="ui-video-quality-menu">
                                            <div class="ui-video-quality-title">Quality</div>
                                            <For each={QUALITY_OPTIONS}>
                                                {option => (
                                                    <button
                                                        class={cn(
                                                            'ui-video-quality-option',
                                                            (props.quality || 'standard') ===
                                                                option.id &&
                                                                'ui-video-quality-option-active'
                                                        )}
                                                        onClick={() =>
                                                            props.onQualityChange?.(option.id)
                                                        }
                                                    >
                                                        <span>{option.label}</span>
                                                        <Show
                                                            when={
                                                                (props.quality || 'standard') ===
                                                                option.id
                                                            }
                                                        >
                                                            <Check size={14} />
                                                        </Show>
                                                    </button>
                                                )}
                                            </For>
                                        </div>
                                    </Popover>
                                </Show>

                                <Tooltip
                                    content={isFullscreen() ? 'Exit Fullscreen' : 'Fullscreen'}
                                >
                                    <Button
                                        variant="ghost"
                                        size="icon-sm"
                                        onClick={e => toggleFullscreen(e)}
                                    >
                                        <Show
                                            when={isFullscreen()}
                                            fallback={<Maximize size={18} />}
                                        >
                                            <Minimize size={18} />
                                        </Show>
                                    </Button>
                                </Tooltip>
                            </div>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    );
};

// Internal AlertCircle icon since I didn't import it
const AlertCircle = (props: { size: number }) => (
    <svg
        width={props.size}
        height={props.size}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
    >
        <circle cx="12" cy="12" r="10" />
        <line x1="12" y1="8" x2="12" y2="12" />
        <line x1="12" y1="16" x2="12.01" y2="16" />
    </svg>
);
