import { Component, Show } from "solid-js";
import { 
    X, Minus, Plus, Maximize, RotateCw, Hand, ChevronLeft, ChevronRight 
} from "lucide-solid";
import { useViewport, useLibrary } from "../../../core/hooks";
import { Button } from "../../ui/Button";
import { Slider } from "../../ui/Slider";
import { ToggleGroup, ToggleGroupItem } from "../../ui/ToggleGroup";
import { ButtonGroup } from "../../ui/ButtonGroup";
import { useViewportContext, FlipState } from "./ViewportContext";
import "./item-view-toolbar.css";
// Props removed as we use context now


export const ItemViewToolbar: Component = () => {
    const viewport = useViewport();
    const lib = useLibrary();
    const { 
        zoom, setZoom, 
        tool, setTool,
        flip, setFlip,
        mediaType
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
                <Button variant="ghost" size="icon" onClick={() => viewport.closeItem()} title="Close (Esc)">
                    <X size={18} />
                </Button>
            </div>

            <Show when={showZoom()}>
                <div class="toolbar-group zoom-controls">
                    <Button variant="ghost" size="icon" onClick={() => setZoom(Math.max(zoom() - 10, 5))}>
                        <Minus size={16} />
                    </Button>
                    
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
                    <Button variant="ghost" size="icon" onClick={() => setZoom(Math.min(zoom() + 10, 500))}>
                        <Plus size={16} />
                    </Button>
                    
                    <div class="toolbar-separator" />
                    
                    <Button variant="ghost" size="sm" onClick={() => setZoom(100)} class="zoom-btn" title="Original Size (Cmd+1)">
                        1:1
                    </Button>
                    <Show when={mediaType() === 'image' || mediaType() === 'unknown'}>
                        <Button variant="ghost" size="icon" onClick={triggerFit} title="Fit Screen (Cmd+0)">
                            <Maximize size={16} />
                        </Button>
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
                        <ToggleGroupItem value="pan" title="Pan Tool (H)">
                            <Hand size={16} />
                        </ToggleGroupItem>
                        <ToggleGroupItem value="rotate" title="Rotate Tool (R)">
                            <RotateCw size={16} />
                        </ToggleGroupItem>
                    </ToggleGroup>

                    <div class="toolbar-separator" />

                    <ToggleGroup type="multiple" value={[]}> 
                        <ToggleGroupItem value="flipH" onClick={toggleFlipH} title="Flip Horizontal" data-state={flip().horizontal ? 'on' : 'off'}>
                            <FlipHorizontalIcon />
                        </ToggleGroupItem>
                        <ToggleGroupItem value="flipV" onClick={toggleFlipV} title="Flip Vertical" data-state={flip().vertical ? 'on' : 'off'}>
                            <FlipVerticalIcon />
                        </ToggleGroupItem>
                    </ToggleGroup>
                </div>
            </Show>

            <div class="toolbar-group">
                <ButtonGroup>
                    <Button variant="ghost" size="icon" onClick={() => navigate(-1)} title="Previous">
                        <ChevronLeft size={18} />
                    </Button>
                    <Button variant="ghost" size="icon" onClick={() => navigate(1)} title="Next">
                        <ChevronRight size={18} />
                    </Button>
                </ButtonGroup>
            </div>
        </div>
    );
};

// Simple Icon Components for cleaner code
const FlipHorizontalIcon = () => (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M8 3H5a2 2 0 0 0-2 2v14c0 1.1.9 2 2 2h3" />
        <path d="M16 3h3a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-3" />
        <path d="M12 3v18" />
    </svg>
);

const FlipVerticalIcon = () => (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M21 8v3a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8" />
        <path d="M21 16v-3a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v3" />
        <path d="M21 12H3" />
    </svg>
);
