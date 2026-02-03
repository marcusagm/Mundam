import { Component, onMount, createSignal, onCleanup } from "solid-js";
import { useItemViewContext } from "../../ItemViewContext";

interface ImageViewerProps {
    src: string;
    alt?: string;
}

export const ImageViewer: Component<ImageViewerProps> = (props) => {
    const { 
        zoom, setZoom, 
        rotation, setRotation, 
        flip,
        tool,
        position, setPosition,
        reset 
    } = useItemViewContext();

    let imgRef: HTMLImageElement | undefined;
    let containerRef: HTMLDivElement | undefined;
    
    // State for drag interactions
    const [isDragging, setIsDragging] = createSignal(false);
    const [startPos, setStartPos] = createSignal({ x: 0, y: 0 });
    const [startRotation, setStartRotation] = createSignal(0);

    // Initial Fit Logic
    const fitToScreen = () => {
        if (!imgRef || !containerRef) return;
        const container = containerRef.getBoundingClientRect();
        const naturalWidth = imgRef.naturalWidth || 800;
        const naturalHeight = imgRef.naturalHeight || 600;

        if (naturalWidth === 0 || naturalHeight === 0) return;

        // Calculate ratios
        const widthRatio = (container.width * 0.9) / naturalWidth;
        const heightRatio = (container.height * 0.9) / naturalHeight;
        
        // Use the smaller ratio to ensure it fits, capped at 100% if desired, 
        // or just fit entirely. User requested "percentage of zoom corresponding to size on open".
        const bestFit = Math.min(widthRatio, heightRatio) * 100;
        
        setZoom(bestFit);
        setPosition({ x: 0, y: 0 });
    };

    // Run fit only when image loads perfectly
    const onImageLoad = () => {
        fitToScreen();
    };

    // Event Listener for Toolbar actions
    onMount(() => {
        const handleFit = () => fitToScreen();
        window.addEventListener('viewport:fit', handleFit);
        onCleanup(() => window.removeEventListener('viewport:fit', handleFit));
    });

    // Mouse Interaction
    const handleMouseDown = (e: MouseEvent) => {
        e.preventDefault();
        setIsDragging(true);

        if (tool() === "pan") {
            setStartPos({ x: e.clientX - position().x, y: e.clientY - position().y });
        } else if (tool() === "rotate") {
            // Calculate initial angle relative to center
            if (!containerRef) return;
            const rect = containerRef.getBoundingClientRect();
            const centerX = rect.left + rect.width / 2;
            const centerY = rect.top + rect.height / 2;
            const angle = Math.atan2(e.clientY - centerY, e.clientX - centerX);
            setStartRotation(angle - (rotation() * Math.PI / 180));
        }
    };

    const handleMouseMove = (e: MouseEvent) => {
        if (!isDragging()) return;

        if (tool() === "pan") {
            setPosition({
                x: e.clientX - startPos().x,
                y: e.clientY - startPos().y
            });
        } else if (tool() === "rotate") {
            if (!containerRef) return;
            const rect = containerRef.getBoundingClientRect();
            const centerX = rect.left + rect.width / 2;
            const centerY = rect.top + rect.height / 2;
            
            const angle = Math.atan2(e.clientY - centerY, e.clientX - centerX);
            const deg = (angle - startRotation()) * 180 / Math.PI;
            setRotation(deg);
        }
    };

    const handleMouseUp = () => {
        setIsDragging(false);
    };

    const handleWheel = (e: WheelEvent) => {
        e.preventDefault();
        const delta = e.deltaY > 0 ? -10 : 10;
        const nextZoom = Math.max(5, Math.min(500, zoom() + delta)); // 5% to 500%
        setZoom(nextZoom);
    };

    // Reset on unmount
    onCleanup(() => {
        reset();
    });

    return (
        <div 
            ref={containerRef}
            class="item-view-viewport"
            onMouseDown={handleMouseDown}
            onMouseMove={handleMouseMove}
            onMouseUp={handleMouseUp}
            onMouseLeave={handleMouseUp}
            onWheel={handleWheel}
            style={{ 
                cursor: tool() === "pan" ? (isDragging() ? "grabbing" : "grab") : "crosshair",
                "width": "100%",
                "height": "100%",
                "overflow": "hidden",
                "display": "flex",
                "align-items": "center",
                "justify-content": "center",
                "position": "relative"
            }}
        >
            <img 
                ref={imgRef}
                src={props.src} 
                alt={props.alt}
                onLoad={onImageLoad}
                draggable={false}
                style={{
                    transform: `translate(${position().x}px, ${position().y}px) rotate(${rotation()}deg) scale(${zoom() / 100}) scaleX(${flip().horizontal ? -1 : 1}) scaleY(${flip().vertical ? -1 : 1})`,
                    "transform-origin": "center",
                    "transition": isDragging() ? "none" : "transform 0.1s ease-out",
                    "max-width": "none",
                    "max-height": "none",
                    "user-select": "none",
                    "pointer-events": "none" // Pass events to container
                }}
            />
        </div>
    );
};
