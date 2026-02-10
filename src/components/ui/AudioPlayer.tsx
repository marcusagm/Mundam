import {
    Component,
    createSignal,
    Show,
    For,
    createEffect,
    createMemo,
    createUniqueId,
    untrack
} from 'solid-js';
import { invoke } from '@tauri-apps/api/core';
import { cn } from '../../lib/utils';
import { Button } from './Button';
import { Slider } from './Slider';
import { Loader } from './Loader';
import {
    Play,
    Pause,
    Volume2,
    VolumeX,
    SkipBack,
    SkipForward,
    Music,
    AlertCircle,
    Repeat
} from 'lucide-solid';
import { audioState, audioActions } from '../../core/store/audioStore';
import { videoActions } from '../../core/store/videoStore';
import { createHlsPlayer } from '../../lib/hls-player';
import './audio-player.css';

export interface AudioPlayerProps {
    src: string;
    filePath?: string; // Original system path for waveform extraction
    variant?: 'full' | 'compact';
    autoPlay?: boolean;
    title?: string;
    subtitle?: string;
    onEnded?: () => void;
    onError?: (error: string) => void;
    class?: string;
}

export const AudioPlayer: Component<AudioPlayerProps> = props => {
    const [audioRef, setAudioRef] = createSignal<HTMLAudioElement>();
    const [isPlaying, setIsPlaying] = createSignal(false);
    const [currentTime, setCurrentTime] = createSignal(0);
    const [duration, setDuration] = createSignal(0);

    const [isWaveformLoading, setIsWaveformLoading] = createSignal(false);
    // const [playbackRate, setPlaybackRate] = createSignal(1);
    const [buffered, setBuffered] = createSignal(0);
    const [waveform, setWaveform] = createSignal<number[]>([]);
    const [lastAction, setLastAction] = createSignal<'play' | 'pause' | null>(null);
    const playerId = createUniqueId();

    // HLS Player Hook
    const hlsPlayer = createHlsPlayer(audioRef, () => props.src, { autoStartLoad: true });

    // Single Active Player Logic
    createEffect(() => {
        const activeId = audioState.activePlayerId();
        const ref = audioRef();
        // Use untrack so this effect only runs when activeId changes
        if (activeId && activeId !== playerId && untrack(() => isPlaying())) {
            // Another player started playing, pause this one
            if (ref) {
                ref.pause();
                setIsPlaying(false);
            }
        }
    });

    // Derived loading state
    const isActuallyLoading = () => hlsPlayer.isLoading() || isWaveformLoading();

    // Native error state (for non-HLS playback)
    const [nativeError, setNativeError] = createSignal<string | null>(null);

    // Combined error state
    const error = () => hlsPlayer.errorMessage() || nativeError();

    // Downsample waveform for better UI performance and adaptation
    const displayWaveform = createMemo(() => {
        const data = waveform();
        if (data.length === 0) return [];

        const targetCount = props.variant === 'compact' ? 80 : 250;
        if (data.length <= targetCount) return data;

        const step = data.length / targetCount;
        const result = [];
        for (let i = 0; i < targetCount; i++) {
            result.push(data[Math.floor(i * step)]);
        }
        return result;
    });

    // Reset state and Load Waveform Data when src/path changes
    createEffect(async () => {
        const path = props.filePath;
        // const src = props.src;

        // Reset state on source change
        setNativeError(null);
        setIsWaveformLoading(true);
        setWaveform([]);
        setCurrentTime(0);
        setBuffered(0);

        if (!path) {
            setIsWaveformLoading(false);
            return;
        }

        try {
            // Create a timeout promise
            const timeoutPromise = new Promise<number[]>((_, reject) => {
                setTimeout(() => reject(new Error('timeout')), 15000);
            });

            // Race the invoke against the timeout
            const data = await Promise.race([
                invoke<number[]>('get_audio_waveform_data', { path }),
                timeoutPromise
            ]);

            setWaveform(data);
        } catch (e) {
            console.error('Failed to load waveform:', e);
            setWaveform([]);
        } finally {
            setIsWaveformLoading(false);
        }
    });

    const handleRetry = () => {
        setNativeError(null);
        const ref = audioRef();
        if (hlsPlayer.isHlsActive()) {
            hlsPlayer.getManager()?.startLoad();
        } else if (ref) {
            ref.load();
        }
    };

    const togglePlay = (e?: MouseEvent) => {
        if (e) e.stopPropagation();
        const ref = audioRef();
        if (!ref) return;

        if (ref.paused) {
            ref.play();
            setLastAction('play');
        } else {
            ref.pause();
            setLastAction('pause');
        }
        setTimeout(() => setLastAction(null), 600);
    };

    const formatTime = (seconds: number) => {
        if (isNaN(seconds)) return '00:00';
        const h = Math.floor(seconds / 3600);
        const m = Math.floor((seconds % 3600) / 60);
        const s = Math.floor(seconds % 60);
        const parts = [m, s].map(v => v.toString().padStart(2, '0'));
        if (h > 0) parts.unshift(h.toString());
        return parts.join(':');
    };

    const handleTimeUpdate = () => {
        const ref = audioRef();
        if (!ref) return;
        setCurrentTime(ref.currentTime);
    };

    const updateBuffered = () => {
        const ref = audioRef();
        if (!ref) return;
        const b = ref.buffered;
        const cur = ref.currentTime;
        for (let i = 0; i < b.length; i++) {
            if (b.start(i) <= cur && b.end(i) >= cur) {
                setBuffered(b.end(i));
                return;
            }
        }
    };

    const handleLoadedMetadata = () => {
        const ref = audioRef();
        if (!ref) return;
        setDuration(ref.duration);
    };

    const handleVolumeChange = (val: number) => {
        const ref = audioRef();
        if (!ref) return;
        const v = val / 100;
        audioActions.setVolume(v);
        ref.volume = v;
        if (v > 0) audioActions.setIsMuted(false);
    };

    const skip = (seconds: number) => {
        const ref = audioRef();
        if (!ref) return;
        ref.currentTime = Math.min(Math.max(ref.currentTime + seconds, 0), duration());
    };

    const handleKeyDown = (e: KeyboardEvent) => {
        switch (e.code) {
            case 'Space':
                e.preventDefault();
                togglePlay();
                break;
            case 'ArrowLeft':
                e.preventDefault();
                skip(-5);
                break;
            case 'ArrowRight':
                e.preventDefault();
                skip(5);
                break;
            case 'KeyM':
                e.preventDefault();
                audioActions.setIsMuted(!audioState.isMuted());
                const ref = audioRef();
                if (ref) ref.muted = audioState.isMuted();
                break;
        }
    };

    return (
        <div
            class={cn(
                'ui-audio-player',
                props.variant === 'compact' && 'ui-audio-player-compact',
                props.class
            )}
            onKeyDown={handleKeyDown}
            tabindex="0"
        >
            <audio
                ref={setAudioRef}
                // src is managed by hlsPlayer
                autoplay={props.autoPlay}
                loop={audioState.isLooping()}
                preload="auto"
                onPlay={() => {
                    setIsPlaying(true);
                    audioActions.setActivePlayer(playerId);
                    videoActions.setActivePlayer('audio-player');
                }}
                onPause={() => setIsPlaying(false)}
                onTimeUpdate={handleTimeUpdate}
                onProgress={updateBuffered}
                onLoadedMetadata={handleLoadedMetadata}
                onWaiting={() => {
                    // Handled by hlsPlayer mostly, but can rely on native events too
                }}
                onPlaying={() => {
                    // Handled by hlsPlayer mostly
                }}
                onEnded={props.onEnded}
                onCanPlay={() => {
                    const ref = audioRef();
                    if (ref) ref.volume = audioState.volume();
                }}
                onError={() => {
                    if (!hlsPlayer.isHlsActive()) {
                        const msg = 'Error loading audio file';
                        setNativeError(msg);
                        props.onError?.(msg);
                    }
                }}
            />

            <Show when={error()}>
                <div class="ui-audio-error">
                    <AlertCircle size={48} />
                    <p>{error()}</p>
                    <Button variant="outline" onClick={handleRetry}>
                        Retry
                    </Button>
                </div>
            </Show>

            <Show when={!error()}>
                <Show when={props.variant === 'full'}>
                    <div class="ui-audio-view">
                        <div class="ui-audio-art">
                            <Music size={80} strokeWidth={1.5} />
                        </div>
                        <div class="ui-audio-info">
                            <div class="ui-audio-title">{props.title || 'Unknown Track'}</div>
                            <div class="ui-audio-subtitle">
                                {props.subtitle || 'Unknown Format'}
                            </div>
                        </div>

                        <div class="ui-audio-center-action">
                            <Show when={lastAction() === 'play'}>
                                <div class="ui-audio-center-action-pulse">
                                    <Play size={48} fill="currentColor" />
                                </div>
                            </Show>
                            <Show when={lastAction() === 'pause'}>
                                <div class="ui-audio-center-action-pulse">
                                    <Pause size={48} fill="currentColor" />
                                </div>
                            </Show>
                        </div>
                    </div>
                </Show>

                <div class="ui-audio-controls">
                    <div class="ui-audio-seekbar-container">
                        <div class="ui-audio-waveform">
                            <Show
                                when={displayWaveform().length > 0}
                                fallback={
                                    <div
                                        class="ui-audio-waveform-bar"
                                        style={{ width: '100%', height: '2px' }}
                                    />
                                }
                            >
                                <For each={displayWaveform()}>
                                    {(val, index) => {
                                        const isPlayed = () => {
                                            const prog = (currentTime() / duration()) * 100;
                                            const ptProg =
                                                (index() / (displayWaveform().length || 1)) * 100;
                                            return ptProg <= prog;
                                        };

                                        return (
                                            <div
                                                class={cn(
                                                    'ui-audio-waveform-bar',
                                                    isPlayed() && 'is-played'
                                                )}
                                                style={{
                                                    height: `${Math.max(15, val * 100)}%`,
                                                    opacity:
                                                        index() / (displayWaveform().length || 1) >
                                                        buffered() / duration()
                                                            ? 0.3
                                                            : 1
                                                }}
                                            />
                                        );
                                    }}
                                </For>
                            </Show>
                        </div>
                        <Slider
                            min={0}
                            max={duration()}
                            step={0.1}
                            value={currentTime()}
                            onValueChange={v => {
                                const ref = audioRef();
                                if (ref) ref.currentTime = v;
                            }}
                            class="ui-audio-seekbar-slider"
                        />
                    </div>

                    <div class="ui-audio-controls-row">
                        <div class="ui-audio-controls-left">
                            <Show when={props.variant === 'full'}>
                                <div class="ui-audio-volume-group">
                                    <Button
                                        variant="ghost"
                                        size="icon-sm"
                                        onClick={() => {
                                            const muted = !audioState.isMuted();
                                            audioActions.setIsMuted(muted);
                                            const ref = audioRef();
                                            if (ref) ref.muted = muted;
                                        }}
                                    >
                                        <Show
                                            when={audioState.isMuted() || audioState.volume() === 0}
                                            fallback={<Volume2 size={18} />}
                                        >
                                            <VolumeX size={18} />
                                        </Show>
                                    </Button>
                                    <Slider
                                        min={0}
                                        max={100}
                                        value={audioState.isMuted() ? 0 : audioState.volume() * 100}
                                        onValueChange={handleVolumeChange}
                                        class="ui-audio-volume-slider"
                                    />
                                </div>
                            </Show>
                        </div>

                        <div class="ui-audio-controls-center ui-audio-controls-row">
                            <Show when={props.variant === 'full'}>
                                <Button variant="ghost" size="icon-sm" onClick={() => skip(-5)}>
                                    <SkipBack size={18} fill="currentColor" />
                                </Button>
                            </Show>
                            <Button variant="ghost" size="icon-sm" onClick={togglePlay}>
                                <Show
                                    when={isPlaying()}
                                    fallback={<Play size={24} fill="currentColor" />}
                                >
                                    <Pause size={24} fill="currentColor" />
                                </Show>
                            </Button>
                            <Show when={props.variant === 'full'}>
                                <Button variant="ghost" size="icon-sm" onClick={() => skip(5)}>
                                    <SkipForward size={18} fill="currentColor" />
                                </Button>
                            </Show>
                        </div>

                        <div class="ui-audio-controls-right">
                            <Show when={props.variant === 'full'}>
                                <Button
                                    variant="ghost"
                                    size="icon-sm"
                                    onClick={() =>
                                        audioActions.setIsLooping(!audioState.isLooping())
                                    }
                                    class={cn(audioState.isLooping() && 'text-primary')}
                                >
                                    <Repeat size={18} />
                                </Button>
                            </Show>
                            <div class="ui-audio-time">
                                {formatTime(currentTime())} / {formatTime(duration())}
                            </div>
                        </div>
                    </div>
                </div>
            </Show>

            <Show when={isActuallyLoading() && !error()}>
                <div class="ui-audio-loader">
                    <Loader size={props.variant === 'full' ? 'lg' : 'sm'} />
                </div>
            </Show>
        </div>
    );
};
