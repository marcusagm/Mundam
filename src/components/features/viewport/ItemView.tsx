import { Component, createSignal, createMemo, Show, onMount, onCleanup } from "solid-js";
import { useViewport, useLibrary } from "../../../core/hooks";
import { ItemViewToolbar } from "./ItemViewToolbar";
import "./item-view.css";

export const ItemView: Component = () => {
    const viewport = useViewport();
    const lib = useLibrary();
    
    // Zoom 0 means "Fit Screen"
    const [zoom, setZoom] = createSignal(0); 
    const [tool, setTool] = createSignal<"pan" | "rotate">("pan");
    const [rotation, setRotation] = createSignal(0);
    const [position, setPosition] = createSignal({ x: 0, y: 0 });
    
    // Drag state
    const [isDragging, setIsDragging] = createSignal(false);
    const [startPos, setStartPos] = createSignal({ x: 0, y: 0 });

    const item = createMemo(() => lib.items.find(i => i.id.toString() === viewport.activeItemId()));

    const handleMouseDown = (e: MouseEvent) => {
        if (tool() === "pan") {
            setIsDragging(true);
            setStartPos({ x: e.clientX - position().x, y: e.clientY - position().y });
        } else if (tool() === "rotate") {
            // Rotate tool could be click to rotate 90 deg?
            setRotation(prev => (prev + 90) % 360);
        }
    };

    const handleMouseMove = (e: MouseEvent) => {
        if (isDragging() && tool() === "pan") {
            setPosition({
                x: e.clientX - startPos().x,
                y: e.clientY - startPos().y
            });
        }
    };

    const handleMouseUp = () => {
        setIsDragging(false);
    };

    const handleWheel = (e: WheelEvent) => {
        // Zoom on Ctrl+Wheel or just Wheel
        e.preventDefault();
        const delta = e.deltaY > 0 ? -10 : 10;
        const currentZoom = zoom() === 0 ? 100 : zoom();
        const nextZoom = Math.max(10, Math.min(500, currentZoom + delta));
        setZoom(nextZoom);
    };

    // Reset position when item changes
    onMount(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.key === "Escape") viewport.closeItem();
        };
        window.addEventListener("keydown", handleKeyDown);
        onCleanup(() => window.removeEventListener("keydown", handleKeyDown));
    });

    return (
        <div class="item-view-overlay" onWheel={handleWheel}>
            <ItemViewToolbar 
                zoom={zoom() === 0 ? 100 : zoom()} 
                setZoom={setZoom} 
                tool={tool()} 
                setTool={setTool}
            />
            
            <div 
                class="item-view-viewport"
                onMouseDown={handleMouseDown}
                onMouseMove={handleMouseMove}
                onMouseUp={handleMouseUp}
                onMouseLeave={handleMouseUp}
                style={{ cursor: tool() === "pan" ? (isDragging() ? "grabbing" : "grab") : "crosshair" }}
            >
                <Show when={item()} fallback={<div class="item-error">Asset not found</div>}>
                    <div class="viewer-container">
                        <img 
                            src={`orig://localhost/${item()!.path}`} 
                            alt={item()!.filename}
                            class="viewer-image"
                            style={{
                                transform: `translate(-50%, -50%) translate(${position().x}px, ${position().y}px) scale(${zoom() === 0 ? 1 : zoom() / 100}) rotate(${rotation()}deg)`,
                                "max-width": zoom() === 0 ? "90%" : "none",
                                "max-height": zoom() === 0 ? "90%" : "none",
                                "object-fit": "contain"
                            }}
                            draggable={false}
                        />
                    </div>
                </Show>
            </div>
        </div>
    );
};
