import { createSignal, createEffect, createMemo, createUniqueId, untrack } from 'solid-js';
import { invoke } from '@tauri-apps/api/core';
import { audioState, audioActions } from '../../../core/store/audioStore';
import { createHlsPlayer } from '../../../lib/hls-player';
import { AudioPlayerProps } from './types';

export function useAudioPlayer(props: AudioPlayerProps) {
    const [audioRef, setAudioRef] = createSignal<HTMLAudioElement>();
    const [isPlaying, setIsPlaying] = createSignal(false);
    const [currentTime, setCurrentTime] = createSignal(0);
    const [duration, setDuration] = createSignal(0);
    const [buffered, setBuffered] = createSignal(0);

    const [isWaveformLoading, setIsWaveformLoading] = createSignal(false);
    const [waveform, setWaveform] = createSignal<number[]>([]);
    const [lastAction, setLastAction] = createSignal<'play' | 'pause' | null>(null);
    const [nativeError, setNativeError] = createSignal<string | null>(null);

    const playerId = createUniqueId();

    // HLS Player Hook
    const hlsPlayer = createHlsPlayer(audioRef, () => props.src, { autoStartLoad: true });

    // Single Active Player Logic
    createEffect(() => {
        const activeId = audioState.activePlayerId();
        const ref = audioRef();
        if (activeId && activeId !== playerId && untrack(() => isPlaying())) {
            if (ref) {
                ref.pause();
                setIsPlaying(false);
            }
        }
    });

    // Combined states
    const isActuallyLoading = () => hlsPlayer.isLoading() || isWaveformLoading();
    const error = () => hlsPlayer.errorMessage() || nativeError();

    // Waveform processing
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

    // Reset state and Load Waveform Data when filePath changes
    createEffect(async () => {
        const path = props.filePath;

        // Reset state on path change
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
            const timeoutPromise = new Promise<number[]>((_, reject) => {
                setTimeout(() => reject(new Error('timeout')), 15000);
            });

            const data = await Promise.race([
                invoke<number[]>('get_audio_waveform_data', { path }),
                timeoutPromise
            ]);

            setWaveform(data);
        } catch (e) {
            console.error('AudioPlayer: Failed to load waveform:', e);
            setWaveform([]);
        } finally {
            setIsWaveformLoading(false);
        }
    });

    // Methods
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
                const muted = !audioState.isMuted();
                audioActions.setIsMuted(muted);
                const ref = audioRef();
                if (ref) ref.muted = muted;
                break;
        }
    };

    return {
        playerId,
        audioRef, setAudioRef,
        isPlaying, setIsPlaying,
        currentTime, duration, buffered,
        displayWaveform,
        isActuallyLoading,
        error, setNativeError,
        lastAction,
        togglePlay,
        handleRetry,
        handleTimeUpdate,
        updateBuffered,
        handleLoadedMetadata,
        handleVolumeChange,
        skip,
        handleKeyDown,
        hlsPlayer
    };
}
