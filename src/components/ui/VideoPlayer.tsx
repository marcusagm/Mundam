import { Component, createSignal, onMount, onCleanup, Show } from 'solid-js';
import { cn } from '../../lib/utils';
import { Button } from './Button';
import { Slider } from './Slider';
import { Tooltip } from './Tooltip';
import { Loader } from './Loader';
import {
    Play,
    Pause,
    Volume2,
    VolumeX,
    Maximize,
    Minimize,
    SkipBack,
    SkipForward
} from 'lucide-solid';
import './video-player.css';

export interface VideoPlayerProps {
    src: string;
    variant?: 'full' | 'compact';
    autoPlay?: boolean;
    title?: string;
    onEnded?: () => void;
    onError?: (error: string) => void;
    class?: string;
}

export const VideoPlayer: Component<VideoPlayerProps> = props => {
    let videoRef: HTMLVideoElement | undefined;
    let containerRef: HTMLDivElement | undefined;

    const [isPlaying, setIsPlaying] = createSignal(false);
    const [currentTime, setCurrentTime] = createSignal(0);
    const [duration, setDuration] = createSignal(0);
    const [volume, setVolume] = createSignal(1);
    const [isMuted, setIsMuted] = createSignal(false);
    const [isLoading, setIsLoading] = createSignal(true);
    const [isFullscreen, setIsFullscreen] = createSignal(false);
    const [showControls, setShowControls] = createSignal(true);
    const [error, setError] = createSignal<string | null>(null);
    const [playbackRate, setPlaybackRate] = createSignal(1);
    const [buffered, setBuffered] = createSignal(0);
    const [previewTime, setPreviewTime] = createSignal<number | null>(null);
    const [previewPos, setPreviewPos] = createSignal(0);
    const [lastAction, setLastAction] = createSignal<'play' | 'pause' | null>(null);

    let controlsTimeout: number;

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
        const newMuted = !isMuted();
        setIsMuted(newMuted);
        videoRef.muted = newMuted;
    };

    const handleVolumeChange = (val: number) => {
        if (!videoRef) return;
        const v = val / 100;
        setVolume(v);
        videoRef.volume = v;
        if (v > 0) setIsMuted(false);
    };

    const cyclePlaybackRate = (e?: MouseEvent) => {
        if (e) e.stopPropagation();
        if (!videoRef) return;
        const rates = [1, 1.25, 1.5, 2];
        const current = playbackRate();
        const next = rates[(rates.indexOf(current) + 1) % rates.length];
        setPlaybackRate(next);
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
                handleVolumeChange(Math.min((volume() + 0.1) * 100, 100));
                break;
            case 'ArrowDown':
                e.preventDefault();
                handleVolumeChange(Math.max((volume() - 0.1) * 100, 0));
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
            <Show when={error()}>
                <div class="ui-video-error">
                    <AlertCircle size={48} />
                    <p>{error()}</p>
                    <Button variant="outline" onClick={() => setError(null)}>
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
                        const msg = 'Error loading video format';
                        setError(msg);
                        props.onError?.(msg);
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
                                            when={isMuted() || volume() === 0}
                                            fallback={<Volume2 size={18} />}
                                        >
                                            <VolumeX size={18} />
                                        </Show>
                                    </Button>
                                    <div class="ui-video-volume-slider">
                                        <Slider
                                            min={0}
                                            max={100}
                                            value={isMuted() ? 0 : volume() * 100}
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
                                            {playbackRate()}x
                                        </Button>
                                    </Tooltip>
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
