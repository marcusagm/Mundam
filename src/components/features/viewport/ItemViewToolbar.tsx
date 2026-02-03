import { Component, Show, For } from "solid-js";
import { 
    X, RotateCw, Hand, ChevronLeft, ChevronRight,
    Play, Pause, Clock,
    FlipVertical2,
    FlipHorizontal2,
    ZoomIn,
    ZoomOut,
    Fullscreen,
    Maximize2
} from "lucide-solid";
import { useViewport, useLibrary } from "../../../core/hooks";
import { Button } from "../../ui/Button";
import { Slider } from "../../ui/Slider";
import { ToggleGroup, ToggleGroupItem } from "../../ui/ToggleGroup";
import { ButtonGroup } from "../../ui/ButtonGroup";
import { useViewportContext, FlipState } from "./ViewportContext";
import { Tooltip } from "../../ui/Tooltip";
import { Kbd } from "../../ui/Kbd";
import { shortcutStore } from "../../../core/input";
import { getShortcutDisplayParts } from "../../../core/input/normalizer";
import "./item-view-toolbar.css";

const ShortcutHint: Component<{ name: string; scope?: string }> = (props) => {
    const shortcut = () => shortcutStore.getByNameAndScope(props.name, props.scope || 'image-viewer');
    const parts = () => {
        const s = shortcut();
        if (!s) return [];
        const keys = Array.isArray(s.keys) ? s.keys[0] : s.keys;
        return getShortcutDisplayParts(keys);
    };
    
    return (
        <div class="flex items-center gap-3">
            <span>{props.name}</span>
            <Show when={parts().length > 0}>
                <div class="flex gap-1">
                    <For each={parts()}>
                        {part => <Kbd class="text-[10px] h-5 px-1.5 min-w-[20px]">{part}</Kbd>}
                    </For>
                </div>
            </Show>
        </div>
    );
};

export const ItemViewToolbar: Component = () => {
    const viewport = useViewport();
    const lib = useLibrary();
    const { 
        zoom, setZoom, 
        tool, setTool,
        flip, setFlip,
        mediaType,
        slideshowPlaying, setSlideshowPlaying,
        slideshowDuration, setSlideshowDuration
    } = useViewportContext();

    // Lógica básica para navegar entre itens da lib
    const navigate = (direction: number) => {
        const items = lib.items;
        const currentId = viewport.activeItemId();
        const currentIndex = items.findIndex(i => i.id.toString() === currentId);
        
        if (currentIndex !== -1) {
            const nextIndex = (currentIndex + direction + items.length) % items.length;
            viewport.openItem(items[nextIndex].id.toString());
        }
    };

    const toggleFlipH = () => setFlip((f: FlipState) => ({ ...f, horizontal: !f.horizontal }));
    const toggleFlipV = () => setFlip((f: FlipState) => ({ ...f, vertical: !f.vertical }));

    const triggerFit = () => {
        window.dispatchEvent(new CustomEvent('viewport:fit'));
    };

    // Derived states for UI logic
    const showZoom = () => ['image', 'font', 'unknown', 'model'].includes(mediaType());
    const showTools = () => ['image', 'unknown'].includes(mediaType());

    return (
        <div class="item-view-toolbar">
            <div class="toolbar-group">
                <Tooltip position="bottom" content={<ShortcutHint name="Close Viewer" />}>
                    <Button variant="ghost" size="icon" onClick={() => viewport.closeItem()}>
                        <X size={18} />
                    </Button>
                </Tooltip>
            </div>

            <Show when={showZoom()}>
                <div class="toolbar-group zoom-controls">
                    <Tooltip position="bottom" content={<ShortcutHint name="Zoom Out" />}>
                        <Button variant="ghost" size="icon" onClick={() => setZoom(Math.max(zoom() - 10, 5))}>
                            <ZoomOut size={16} />
                        </Button>
                    </Tooltip>
                    
                    <div class="zoom-display" style={{ "min-width": "40px", "text-align": "center", "font-size": "12px" }}>
                        {Math.round(zoom())}%
                    </div>

                    <Slider 
                        value={zoom()} 
                        min={5} 
                        max={500} 
                        onValueChange={(val) => setZoom(val)}
                        class="zoom-slider"
                    />
                    <Tooltip position="bottom" content={<ShortcutHint name="Zoom In" />}>
                        <Button variant="ghost" size="icon" onClick={() => setZoom(Math.min(zoom() + 10, 500))}>
                            <ZoomIn size={16} />
                        </Button>
                    </Tooltip>
                    
                    <div class="toolbar-separator" />
                    
                    <Tooltip position="bottom" content={<ShortcutHint name="Original Size" />}>
                        <Button variant="ghost" size="icon" onClick={() => setZoom(100)} class="zoom-btn">
                            <Maximize2 size={16} />
                        </Button>
                    </Tooltip>
                    <Show when={mediaType() === 'image' || mediaType() === 'unknown'}>
                        <Tooltip position="bottom" content={<ShortcutHint name="Fit to Screen" />}>
                            <Button variant="ghost" size="icon" onClick={triggerFit}>
                                <Fullscreen size={16} />
                            </Button>
                        </Tooltip>
                    </Show>
                </div>
            </Show>

            <Show when={showTools()}>
                <div class="toolbar-group">
                    <div class="toolbar-label">Tools</div>
                    <ToggleGroup 
                        type="single" 
                        value={tool()} 
                        onValueChange={(val) => val && setTool(val as "pan" | "rotate")}
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
                            <ToggleGroupItem value="flipH" onClick={toggleFlipH} data-state={flip().horizontal ? 'on' : 'off'}>
                                <FlipHorizontal2 size={16} />
                            </ToggleGroupItem>
                        </Tooltip>
                        <Tooltip position="bottom" content={<ShortcutHint name="Flip Vertical" />}>
                            <ToggleGroupItem value="flipV" onClick={toggleFlipV} data-state={flip().vertical ? 'on' : 'off'}>
                                <FlipVertical2 size={16} />
                            </ToggleGroupItem>
                        </Tooltip>
                    </ToggleGroup>
                </div>
            </Show>

            <div class="toolbar-group">
                <div class="toolbar-label">Timer</div>
                <Tooltip position="bottom" content={<ShortcutHint name="Play/Pause Slideshow" />}>
                    <Button 
                        variant="ghost" 
                        size="icon" 
                        onClick={() => setSlideshowPlaying(!slideshowPlaying())}
                        class={slideshowPlaying() ? "text-primary" : ""}
                    >
                        {slideshowPlaying() ? <Pause size={16} /> : <Play size={16} />}
                    </Button>
                </Tooltip>
                
                <div class="timer-select-wrapper">
                    <Clock size={14} style={{ "margin-right": "4px", "opacity": 0.5 }}/>
                     <select 
                        class="timer-select"
                        value={slideshowDuration()} 
                        onChange={(e) => setSlideshowDuration(parseInt(e.currentTarget.value))}
                    >
                        <option value={2}>2s</option>
                        <option value={5}>5s</option>
                        <option value={10}>10s</option>
                        <option value={30}>30s</option>
                        <option value={60}>1m</option>
                    </select>
                </div>
            </div>

            <div class="toolbar-group">
                <ButtonGroup>
                    <Tooltip position="bottom" content={<ShortcutHint name="Previous Item" />}>
                        <Button variant="ghost" size="icon" onClick={() => navigate(-1)}>
                            <ChevronLeft size={18} />
                        </Button>
                    </Tooltip>
                    <Tooltip position="bottom" content={<ShortcutHint name="Next Item" />}>
                        <Button variant="ghost" size="icon" onClick={() => navigate(1)}>
                            <ChevronRight size={18} />
                        </Button>
                    </Tooltip>
                </ButtonGroup>
            </div>
        </div>
    );
};
