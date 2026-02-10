import { Component, Show } from 'solid-js';
import { Slider } from '../Slider';
import { useVideoContext } from './VideoPlayerContext';
import { formatTime } from './utils';

export const VideoSeekbar: Component = () => {
    const {
        duration,
        currentTime,
        buffered,
        handleSeek,
        setPreviewTime,
        setPreviewPos,
        previewTime,
        previewPos
    } = useVideoContext();

    const handleSeekMouseMove = (e: MouseEvent) => {
        const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
        const pos = (e.clientX - rect.left) / rect.width;
        const time = pos * duration();
        setPreviewTime(time);
        setPreviewPos(pos * 100);
    };

    return (
        <div
            class="ui-video-seekbar-container"
            onMouseMove={handleSeekMouseMove}
            onMouseLeave={() => setPreviewTime(null)}
        >
            <Show when={previewTime() !== null}>
                <div class="ui-video-seekbar-preview" style={{ left: `${previewPos()}%` }}>
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
    );
};
