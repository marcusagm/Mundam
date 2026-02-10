import { Component, Show } from 'solid-js';
import { cn } from '../../../lib/utils';
import { Button } from '../Button';
import { Slider } from '../Slider';
import { Play, Pause, Volume2, VolumeX, SkipBack, SkipForward, Repeat } from 'lucide-solid';
import { audioState, audioActions } from '../../../core/store/audioStore';
import { useAudioContext } from './AudioPlayerContext';
import { formatTime } from './utils';

export const AudioControls: Component = () => {
    const {
        props,
        audioRef,
        isPlaying,
        currentTime,
        duration,
        togglePlay,
        handleVolumeChange,
        skip
    } = useAudioContext();

    return (
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
                    <Show when={isPlaying()} fallback={<Play size={24} fill="currentColor" />}>
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
                        onClick={() => audioActions.setIsLooping(!audioState.isLooping())}
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
    );
};
