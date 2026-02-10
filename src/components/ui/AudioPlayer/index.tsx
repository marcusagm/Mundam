import { Component, Show } from 'solid-js';
import { cn } from '../../../lib/utils';
import { Button } from '../Button';
import { Loader } from '../Loader';
import { Play, Pause, Music, AlertCircle } from 'lucide-solid';
import { audioState, audioActions } from '../../../core/store/audioStore';
import { videoActions } from '../../../core/store/videoStore';
import { AudioProvider, useAudioContext } from './AudioPlayerContext';
import { AudioControls } from './AudioControls';
import { AudioWaveform } from './AudioWaveform';
import { AudioPlayerProps } from './types';
import './audio-player.css';

const AudioPlayerContent: Component = () => {
    const {
        props,
        playerId,
        audioRef,
        setAudioRef,
        setIsPlaying,
        handleTimeUpdate,
        updateBuffered,
        handleLoadedMetadata,
        handleRetry,
        error,
        lastAction,
        isActuallyLoading,
        handleKeyDown
    } = useAudioContext();

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
                // src is managed by hlsPlayer internally via the ref
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
                onEnded={props.onEnded}
                onCanPlay={() => {
                    const ref = audioRef();
                    if (ref) ref.volume = audioState.volume();
                }}
                onError={() => {
                    // hlsPlayer handles its own errors, we only catch native ones here if HLS is not active
                    // This is handled inside useAudioPlayer hook partially but native event is useful
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
                    <AudioWaveform />
                    <AudioControls />
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

export const AudioPlayer: Component<AudioPlayerProps> = props => {
    return (
        <AudioProvider playerProps={props}>
            <AudioPlayerContent />
        </AudioProvider>
    );
};

export * from './types';
