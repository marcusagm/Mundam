import { Component, For, Show } from 'solid-js';
import { cn } from '../../../lib/utils';
import { Slider } from '../Slider';
import { useAudioContext } from './AudioPlayerContext';

export const AudioWaveform: Component = () => {
    const { audioRef, duration, currentTime, buffered, displayWaveform } = useAudioContext();

    return (
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
                                const d = duration();
                                if (d === 0) return false;
                                const prog = (currentTime() / d) * 100;
                                const ptProg = (index() / (displayWaveform().length || 1)) * 100;
                                return ptProg <= prog;
                            };

                            const isBuffered = () => {
                                const d = duration();
                                if (d === 0) return true;
                                return index() / (displayWaveform().length || 1) <= buffered() / d;
                            };

                            return (
                                <div
                                    class={cn('ui-audio-waveform-bar', isPlayed() && 'is-played')}
                                    style={{
                                        height: `${Math.max(15, val * 100)}%`,
                                        opacity: isBuffered() ? 1 : 0.3
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
    );
};
