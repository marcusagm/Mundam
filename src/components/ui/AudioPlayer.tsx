import { Component, createSignal, Show, For, createEffect, createMemo } from 'solid-js';
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
    let audioRef: HTMLAudioElement | undefined;
    const [isPlaying, setIsPlaying] = createSignal(false);
    const [currentTime, setCurrentTime] = createSignal(0);
    const [duration, setDuration] = createSignal(0);

    const [isLoading, setIsLoading] = createSignal(true);
    const [isWaveformLoading, setIsWaveformLoading] = createSignal(false);
    const [error, setError] = createSignal<string | null>(null);
    // const [playbackRate, setPlaybackRate] = createSignal(1);
    const [buffered, setBuffered] = createSignal(0);
    const [waveform, setWaveform] = createSignal<number[]>([]);
    const [lastAction, setLastAction] = createSignal<'play' | 'pause' | null>(null);

    // Derived loading state
    const isActuallyLoading = () => isLoading() || isWaveformLoading();

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
        setError(null);
        setIsLoading(true);
        setIsWaveformLoading(true);
        setWaveform([]);
        setCurrentTime(0);
        setBuffered(0);

        if (!path) {
            setIsWaveformLoading(false);
            return;
        }

        try {
            const data = await invoke<number[]>('get_audio_waveform_data', { path });
            setWaveform(data);
        } catch (e) {
            console.error('Failed to load waveform:', e);
        } finally {
            setIsWaveformLoading(false);
        }
    });

    const handleRetry = () => {
        setError(null);
        setIsLoading(true);
        if (audioRef) {
            audioRef.load();
        }
    };

    const togglePlay = (e?: MouseEvent) => {
        if (e) e.stopPropagation();
        if (!audioRef) return;

        if (audioRef.paused) {
            audioRef.play();
            setLastAction('play');
        } else {
            audioRef.pause();
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
        if (!audioRef) return;
        setCurrentTime(audioRef.currentTime);
    };

    const updateBuffered = () => {
        if (!audioRef) return;
        const b = audioRef.buffered;
        const cur = audioRef.currentTime;
        for (let i = 0; i < b.length; i++) {
            if (b.start(i) <= cur && b.end(i) >= cur) {
                setBuffered(b.end(i));
                return;
            }
        }
    };

    const handleLoadedMetadata = () => {
        if (!audioRef) return;
        setDuration(audioRef.duration);
        setIsLoading(false);
    };

    const handleVolumeChange = (val: number) => {
        if (!audioRef) return;
        const v = val / 100;
        audioActions.setVolume(v);
        audioRef.volume = v;
        if (v > 0) audioActions.setIsMuted(false);
    };

    const skip = (seconds: number) => {
        if (!audioRef) return;
        audioRef.currentTime = Math.min(Math.max(audioRef.currentTime + seconds, 0), duration());
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
                if (audioRef) audioRef.muted = audioState.isMuted();
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
                ref={audioRef}
                src={props.src}
                autoplay={props.autoPlay}
                loop={audioState.isLooping()}
                preload="metadata"
                onPlay={() => setIsPlaying(true)}
                onPause={() => setIsPlaying(false)}
                onTimeUpdate={handleTimeUpdate}
                onProgress={updateBuffered}
                onLoadedMetadata={handleLoadedMetadata}
                onWaiting={() => setIsLoading(true)}
                onPlaying={() => setIsLoading(false)}
                onEnded={props.onEnded}
                onCanPlay={() => {
                    if (audioRef) audioRef.volume = audioState.volume();
                }}
                onError={() => {
                    const msg = 'Error loading audio file';
                    setError(msg);
                    props.onError?.(msg);
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
                                if (audioRef) audioRef.currentTime = v;
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
                                            if (audioRef) audioRef.muted = muted;
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
                    <Loader size="lg" />
                </div>
            </Show>
        </div>
    );
};
