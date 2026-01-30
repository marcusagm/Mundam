import { Component, createSignal, createMemo, Show } from "solid-js";
import { useViewport, useLibrary } from "../../../core/hooks";
import { ItemViewToolbar } from "./ItemViewToolbar";
import { createInputScope, useShortcuts } from "../../../core/input";
import "./item-view.css";

export const ItemView: Component = () => {
    const viewport = useViewport();
    const lib = useLibrary();
    
    // Push image-viewer scope when this component is mounted
    createInputScope('image-viewer');
    
    // Zoom 0 means "Fit Screen"
    const [zoom, setZoom] = createSignal(0); 
    const [tool, setTool] = createSignal<"pan" | "rotate">("pan");
    const [rotation, setRotation] = createSignal(0);
    const [position, setPosition] = createSignal({ x: 0, y: 0 });
    
    // Drag state
    const [isDragging, setIsDragging] = createSignal(false);
    const [startPos, setStartPos] = createSignal({ x: 0, y: 0 });

    const item = createMemo(() => lib.items.find(i => i.id.toString() === viewport.activeItemId()));

    // Navigation helper
    const navigate = (direction: number) => {
        const items = lib.items;
        const currentId = viewport.activeItemId();
        const currentIndex = items.findIndex(i => i.id.toString() === currentId);
        
        if (currentIndex !== -1) {
            const nextIndex = (currentIndex + direction + items.length) % items.length;
            viewport.openItem(items[nextIndex].id.toString());
            // Reset position on navigation
            setPosition({ x: 0, y: 0 });
            setRotation(0);
        }
    };
    
    // Zoom helpers
    const zoomIn = () => setZoom(Math.min((zoom() === 0 ? 100 : zoom()) + 10, 500));
    const zoomOut = () => setZoom(Math.max((zoom() === 0 ? 100 : zoom()) - 10, 10));
    const fitToScreen = () => setZoom(0);
    const originalSize = () => setZoom(100);

    // Register keyboard shortcuts for image-viewer scope
    useShortcuts([
        { keys: 'Escape', name: 'Close Viewer', scope: 'image-viewer', action: () => viewport.closeItem() },
        { keys: 'Equal', name: 'Zoom In', scope: 'image-viewer', action: zoomIn },
        { keys: 'Minus', name: 'Zoom Out', scope: 'image-viewer', action: zoomOut },
        { keys: 'Meta+Digit0', name: 'Fit to Screen', scope: 'image-viewer', action: fitToScreen },
        { keys: 'Meta+Digit1', name: 'Original Size', scope: 'image-viewer', action: originalSize },
        { keys: 'KeyH', name: 'Pan Tool', scope: 'image-viewer', action: () => setTool("pan") },
        { keys: 'KeyR', name: 'Rotate Tool', scope: 'image-viewer', action: () => setTool("rotate") },
        { keys: 'ArrowLeft', name: 'Previous Image', scope: 'image-viewer', action: () => navigate(-1) },
        { keys: 'ArrowRight', name: 'Next Image', scope: 'image-viewer', action: () => navigate(1) },
    ]);

    const handleMouseDown = (e: MouseEvent) => {
        if (tool() === "pan") {
            setIsDragging(true);
            setStartPos({ x: e.clientX - position().x, y: e.clientY - position().y });
        } else if (tool() === "rotate") {
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
        e.preventDefault();
        const delta = e.deltaY > 0 ? -10 : 10;
        const currentZoom = zoom() === 0 ? 100 : zoom();
        const nextZoom = Math.max(10, Math.min(500, currentZoom + delta));
        setZoom(nextZoom);
    };

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

