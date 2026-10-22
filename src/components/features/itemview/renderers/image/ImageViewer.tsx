import { Component, onMount, createSignal, onCleanup } from 'solid-js';
import { useItemViewContext } from '../../ItemViewContext';
import { useViewport, useLibrary } from '../../../../../core/hooks';
import { assetDragSource } from '../../../../../core/dnd';

// Referenced so the `use:assetDragSource` directive is not tree-shaken
void assetDragSource;

const ROTATION_SNAP_STEP = 45;

interface ImageViewerProps {
    src: string;
    alt?: string;
    onImageLoaded?: () => void;
    onImageError?: () => void;
}

export const ImageViewer: Component<ImageViewerProps> = props => {
    const { rotation, setRotation, flip, tool, reset } = useItemViewContext();

    const viewport = useViewport();
    const lib = useLibrary();
    const currentItem = () => lib.items.find(i => i.id.toString() === viewport.activeItemId());

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

        viewport.setZoom(bestFit);
        viewport.setPan(0, 0);
    };

    // Run fit only when image loads perfectly
    const onImageLoad = () => {
        fitToScreen();
        props.onImageLoaded?.();
    };

    const onImageLoadError = () => {
        props.onImageError?.();
    };

    // Event Listener for Toolbar actions
    onMount(() => {
        const handleFit = () => fitToScreen();
        window.addEventListener('viewport:fit', handleFit);
        onCleanup(() => window.removeEventListener('viewport:fit', handleFit));
    });

    // Mouse Interaction
    const handleMouseDown = (e: MouseEvent) => {
        // Select tool keeps native behavior so the image can be dragged out of the app
        if (tool() === 'select') return;
        e.preventDefault();
        setIsDragging(true);

        if (tool() === 'pan') {
            const currentPan = viewport.pan();
            setStartPos({ x: e.clientX - currentPan.x, y: e.clientY - currentPan.y });
        } else if (tool() === 'rotate') {
            // Calculate initial angle relative to center
            if (!containerRef) return;
            const rect = containerRef.getBoundingClientRect();
            const centerX = rect.left + rect.width / 2;
            const centerY = rect.top + rect.height / 2;
            const angle = Math.atan2(e.clientY - centerY, e.clientX - centerX);
            setStartRotation(angle - (rotation() * Math.PI) / 180);
        }
    };

    const handleMouseMove = (e: MouseEvent) => {
        if (!isDragging()) return;

        if (tool() === 'pan') {
            viewport.setPan(e.clientX - startPos().x, e.clientY - startPos().y);
        } else if (tool() === 'rotate') {
            if (!containerRef) return;
            const rect = containerRef.getBoundingClientRect();
            const centerX = rect.left + rect.width / 2;
            const centerY = rect.top + rect.height / 2;

            const angle = Math.atan2(e.clientY - centerY, e.clientX - centerX);
            const deg = ((angle - startRotation()) * 180) / Math.PI;
            setRotation(
                e.shiftKey ? Math.round(deg / ROTATION_SNAP_STEP) * ROTATION_SNAP_STEP : deg
            );
        }
    };

    const handleMouseUp = () => {
        setIsDragging(false);
    };

    const handleWheel = (e: WheelEvent) => {
        e.preventDefault();
        const delta = e.deltaY > 0 ? -10 : 10;
        const nextZoom = Math.max(5, Math.min(500, viewport.zoom() + delta)); // 5% to 500%
        viewport.setZoom(nextZoom);
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
                cursor:
                    tool() === 'select'
                        ? 'default'
                        : tool() === 'pan'
                          ? isDragging()
                              ? 'grabbing'
                              : 'grab'
                          : 'crosshair',
                width: '100%',
                height: '100%',
                overflow: 'hidden',
                display: 'flex',
                'align-items': 'center',
                'justify-content': 'center',
                position: 'relative'
            }}
        >
            <img
                ref={imgRef}
                src={props.src}
                alt={props.alt}
                onLoad={onImageLoad}
                onError={onImageLoadError}
                use:assetDragSource={{
                    id: viewport.activeItemId() ?? '',
                    path: currentItem()?.path ?? '',
                    thumbnailPath: currentItem()?.thumbnail_path ?? null,
                    isSelected: false,
                    getSelectedIds: () => [],
                    getItemInfo: () => currentItem(),
                    enabled: tool() === 'select'
                }}
                style={{
                    transform: `translate(${viewport.pan().x}px, ${viewport.pan().y}px) rotate(${rotation()}deg) scale(${viewport.zoom() / 100}) scaleX(${flip().horizontal ? -1 : 1}) scaleY(${flip().vertical ? -1 : 1})`,
                    'transform-origin': 'center',
                    transition: isDragging() ? 'none' : 'transform 0.1s ease-out',
                    'max-width': 'none',
                    'max-height': 'none',
                    'user-select': 'none',
                    'pointer-events': tool() === 'select' ? 'auto' : 'none' // Other tools pass events to container
                }}
            />
        </div>
    );
};
