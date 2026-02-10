import { Component, Show, For } from 'solid-js';
import { cn } from '../../../lib/utils';
import { Button } from '../Button';
import { Slider } from '../Slider';
import { Tooltip } from '../Tooltip';
import { Popover } from '../Popover';
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
import { videoState } from '../../../core/store/videoStore';
import { useVideoContext } from './VideoPlayerContext';
import { formatTime } from './utils';
import { QUALITY_OPTIONS } from './types';
import { VideoSeekbar } from './VideoSeekbar';

export const VideoControls: Component = () => {
    const {
        props,
        isPlaying,
        togglePlay,
        skip,
        toggleMute,
        handleVolumeChange,
        currentTime,
        duration,
        cyclePlaybackRate,
        isFullscreen,
        toggleFullscreen,
        needsTranscode
    } = useVideoContext();

    return (
        <div class="ui-video-bottom-controls">
            {/* Seek Bar Area */}
            <VideoSeekbar />

            <div class="ui-video-controls-row">
                <div class="ui-video-controls-left ui-video-controls-row">
                    <Tooltip content={isPlaying() ? 'Pause' : 'Play'}>
                        <Button
                            variant="ghost"
                            size="icon-sm"
                            onClick={(e: MouseEvent) => togglePlay(e)}
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
                            <Button variant="ghost" size="icon-sm" onClick={() => skip(-5)}>
                                <SkipBack size={18} fill="currentColor" />
                            </Button>
                        </Tooltip>
                        <Tooltip content="Step forward 5s">
                            <Button variant="ghost" size="icon-sm" onClick={() => skip(5)}>
                                <SkipForward size={18} fill="currentColor" />
                            </Button>
                        </Tooltip>
                    </Show>

                    <div class="ui-video-volume-group">
                        <Button variant="ghost" size="icon-sm" onClick={e => toggleMute(e)}>
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
                                value={videoState.isMuted() ? 0 : videoState.volume() * 100}
                                onValueChange={handleVolumeChange}
                            />
                        </div>
                    </div>

                    <div class="ui-video-time">
                        {formatTime(currentTime())} <span>/</span> {formatTime(duration())}
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
                                                (props.quality || 'standard') === option.id &&
                                                    'ui-video-quality-option-active'
                                            )}
                                            onClick={() => props.onQualityChange?.(option.id)}
                                        >
                                            <span>{option.label}</span>
                                            <Show
                                                when={(props.quality || 'standard') === option.id}
                                            >
                                                <Check size={14} />
                                            </Show>
                                        </button>
                                    )}
                                </For>
                            </div>
                        </Popover>
                    </Show>

                    <Tooltip content={isFullscreen() ? 'Exit Fullscreen' : 'Fullscreen'}>
                        <Button variant="ghost" size="icon-sm" onClick={e => toggleFullscreen(e)}>
                            <Show when={isFullscreen()} fallback={<Maximize size={18} />}>
                                <Minimize size={18} />
                            </Show>
                        </Button>
                    </Tooltip>
                </div>
            </div>
        </div>
    );
};
