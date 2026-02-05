import { Component } from 'solid-js';
import {
    RotateCw,
    Hand,
    Play,
    Pause,
    Clock,
    FlipVertical2,
    FlipHorizontal2,
    ZoomIn,
    ZoomOut,
    Fullscreen,
    Maximize2
} from 'lucide-solid';
import { Button } from '../../../../ui/Button';
import { Slider } from '../../../../ui/Slider';
import { ToggleGroup, ToggleGroupItem } from '../../../../ui/ToggleGroup';
import { Tooltip } from '../../../../ui/Tooltip';
import { useItemViewContext, FlipState } from '../../ItemViewContext';
import { ShortcutHint } from '../../common/ToolbarUtils';
import { Select } from '../../../../ui';

export const ImageToolbar: Component = () => {
    const {
        zoom,
        setZoom,
        tool,
        setTool,
        flip,
        setFlip,
        slideshowPlaying,
        setSlideshowPlaying,
        slideshowDuration,
        setSlideshowDuration
    } = useItemViewContext();

    const toggleFlipH = () => setFlip((f: FlipState) => ({ ...f, horizontal: !f.horizontal }));
    const toggleFlipV = () => setFlip((f: FlipState) => ({ ...f, vertical: !f.vertical }));

    const triggerFit = () => {
        window.dispatchEvent(new CustomEvent('viewport:fit'));
    };

    return (
        <>
            <div class="toolbar-group zoom-controls">
                <Tooltip position="bottom" content={<ShortcutHint name="Zoom Out" />}>
                    <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => setZoom(Math.max(zoom() - 10, 5))}
                    >
                        <ZoomOut size={16} />
                    </Button>
                </Tooltip>

                <div
                    class="zoom-display"
                    style={{ 'min-width': '40px', 'text-align': 'center', 'font-size': '12px' }}
                >
                    {Math.round(zoom())}%
                </div>

                <Slider
                    value={zoom()}
                    min={5}
                    max={500}
                    onValueChange={val => setZoom(val)}
                    class="zoom-slider"
                />
                <Tooltip position="bottom" content={<ShortcutHint name="Zoom In" />}>
                    <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => setZoom(Math.min(zoom() + 10, 500))}
                    >
                        <ZoomIn size={16} />
                    </Button>
                </Tooltip>

                <div class="toolbar-separator" />

                <Tooltip position="bottom" content={<ShortcutHint name="Original Size" />}>
                    <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => setZoom(100)}
                        class="zoom-btn"
                    >
                        <Maximize2 size={16} />
                    </Button>
                </Tooltip>
                <Tooltip position="bottom" content={<ShortcutHint name="Fit to Screen" />}>
                    <Button variant="ghost" size="icon" onClick={triggerFit}>
                        <Fullscreen size={16} />
                    </Button>
                </Tooltip>
            </div>

            <div class="toolbar-group">
                <div class="toolbar-label">Tools</div>
                <ToggleGroup
                    type="single"
                    value={tool()}
                    onValueChange={val => val && setTool(val as 'pan' | 'rotate')}
                >
                    <Tooltip position="bottom" content={<ShortcutHint name="Pan Tool" />}>
                        <ToggleGroupItem value="pan">
                            <Hand size={16} />
                        </ToggleGroupItem>
                    </Tooltip>
                    <Tooltip position="bottom" content={<ShortcutHint name="Rotate Tool" />}>
                        <ToggleGroupItem value="rotate">
                            <RotateCw size={16} />
                        </ToggleGroupItem>
                    </Tooltip>
                </ToggleGroup>

                <div class="toolbar-separator" />

                <ToggleGroup type="multiple" value={[]}>
                    <Tooltip position="bottom" content={<ShortcutHint name="Flip Horizontal" />}>
                        <ToggleGroupItem
                            value="flipH"
                            onClick={toggleFlipH}
                            data-state={flip().horizontal ? 'on' : 'off'}
                        >
                            <FlipHorizontal2 size={16} />
                        </ToggleGroupItem>
                    </Tooltip>
                    <Tooltip position="bottom" content={<ShortcutHint name="Flip Vertical" />}>
                        <ToggleGroupItem
                            value="flipV"
                            onClick={toggleFlipV}
                            data-state={flip().vertical ? 'on' : 'off'}
                        >
                            <FlipVertical2 size={16} />
                        </ToggleGroupItem>
                    </Tooltip>
                </ToggleGroup>
            </div>

            <div class="toolbar-group">
                <div class="toolbar-label">Timer</div>
                <Tooltip position="bottom" content={<ShortcutHint name="Play/Pause Slideshow" />}>
                    <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => setSlideshowPlaying(!slideshowPlaying())}
                        class={slideshowPlaying() ? 'text-primary' : ''}
                    >
                        {slideshowPlaying() ? <Pause size={16} /> : <Play size={16} />}
                    </Button>
                </Tooltip>

                <div class="timer-select-wrapper">
                    <Select
                        size="sm"
                        leftIcon={<Clock />}
                        value={String(slideshowDuration())}
                        onValueChange={val => setSlideshowDuration(Number(val))}
                        options={[
                            { value: '5', label: '5s' },
                            { value: '10', label: '10s' },
                            { value: '30', label: '30s' },
                            { value: '60', label: '1m' },
                            { value: '120', label: '2m' },
                            { value: '300', label: '5m' },
                            { value: '600', label: '10m' },
                            { value: '1800', label: '30m' },
                            { value: '3600', label: '1h' }
                        ]}
                    />
                </div>
            </div>
        </>
    );
};
