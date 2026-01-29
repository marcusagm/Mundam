import { Component } from "solid-js";
import { 
    X, 
    Minus, 
    Plus, 
    Maximize, 
    RotateCw, 
    Hand, 
    ChevronLeft, 
    ChevronRight
} from "lucide-solid";
import { useViewport, useLibrary } from "../../../core/hooks";
import { Button } from "../../ui/Button";
import { Slider } from "../../ui/Slider";
import { ToggleGroup, ToggleGroupItem } from "../../ui/ToggleGroup";
import { ButtonGroup } from "../../ui/ButtonGroup";
import "./item-view-toolbar.css";

interface ItemViewToolbarProps {
    zoom: number;
    setZoom: (val: number) => void;
    tool: "pan" | "rotate";
    setTool: (tool: "pan" | "rotate") => void;
}

export const ItemViewToolbar: Component<ItemViewToolbarProps> = (props) => {
    const viewport = useViewport();
    const lib = useLibrary();

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

    return (
        <div class="item-view-toolbar">
            <div class="toolbar-group">
                <Button variant="ghost" size="icon" onClick={() => viewport.closeItem()} title="Close">
                    <X size={18} />
                </Button>
            </div>

            <div class="toolbar-group zoom-controls">
                <Button variant="ghost" size="icon" onClick={() => props.setZoom(Math.max(props.zoom - 10, 10))}>
                    <Minus size={16} />
                </Button>
                <Slider 
                    value={props.zoom} 
                    min={10} 
                    max={300} 
                    onValueChange={(val) => props.setZoom(val)}
                    class="zoom-slider"
                />
                <Button variant="ghost" size="icon" onClick={() => props.setZoom(Math.min(props.zoom + 10, 300))}>
                    <Plus size={16} />
                </Button>
                
                <div class="toolbar-separator" />
                
                <Button variant="ghost" size="sm" onClick={() => props.setZoom(100)} class="zoom-btn">
                    1:1
                </Button>
                <Button variant="ghost" size="icon" onClick={() => props.setZoom(0)} title="Fit Screen">
                    <Maximize size={16} />
                </Button>
            </div>

            <div class="toolbar-group">
                <ToggleGroup 
                    type="single" 
                    value={props.tool} 
                    onValueChange={(val) => val && props.setTool(val as "pan" | "rotate")}
                >
                    <ToggleGroupItem value="pan" title="Pan Tool">
                        <Hand size={16} />
                    </ToggleGroupItem>
                    <ToggleGroupItem value="rotate" title="Rotate Tool">
                        <RotateCw size={16} />
                    </ToggleGroupItem>
                </ToggleGroup>
            </div>

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
