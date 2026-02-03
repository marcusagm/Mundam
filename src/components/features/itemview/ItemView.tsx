import { Component, createMemo, Show, Switch, Match, createEffect, onCleanup, onMount } from "solid-js";
import { useViewport, useLibrary } from "../../../core/hooks";
import { useShortcuts, createConditionalScope } from "../../../core/input";
import { ItemViewProvider, useItemViewContext, FlipState } from "./ItemViewContext";
import { BaseToolbar } from "./common/BaseToolbar";
import { ImageToolbar } from "./renderers/image/ImageToolbar";
import { FontToolbar } from "./renderers/font/FontToolbar";
import { ImageViewer } from "./renderers/image/ImageViewer";
import { VideoPlayer } from "./renderers/video/VideoPlayer";
import { FontView } from "./renderers/font/FontView";
import { ModelViewer } from "./renderers/model/ModelViewer";
import "./item-view.css";

// Helper to determine media type from extension
const getMediaType = (filename: string): 'image' | 'video' | 'audio' | 'font' | 'model' | 'unknown' => {
    const ext = filename.split('.').pop()?.toLowerCase();
    if (!ext) return 'unknown';

    const imageExts = ['jpg', 'jpeg', 'jpe', 'jfif', 'png', 'webp', 'gif', 'bmp', 'ico', 'svg', 'avif']; 
    const videoExts = ['mp4', 'm4v', 'webm', 'mov', 'qt', 'mxf', 'mkv'];
    const audioExts = ['mp3', 'wav', 'ogg', 'aac', 'flac', 'm4a'];
    const fontExts = ['ttf', 'otf', 'woff', 'woff2'];
    const modelExts = ['blend', 'fbx', 'obj', 'glb', 'gltf', 'stl', 'dae', '3ds', 'dxf', 'lwo', 'lws'];

    if (imageExts.includes(ext)) return 'image';
    if (videoExts.includes(ext)) return 'video';
    if (audioExts.includes(ext)) return 'audio';
    if (fontExts.includes(ext)) return 'font';
    if (modelExts.includes(ext)) return 'model';
    
    return 'unknown';
};

export const ItemView: Component = () => {
    return (
        <ItemViewProvider>
            <ItemViewContent />
        </ItemViewProvider>
    );
};

const ItemViewContent: Component = () => {
    const viewport = useViewport();
    const lib = useLibrary();
    const { 
        reset, setMediaType, mediaType,
        slideshowPlaying, slideshowDuration, setSlideshowPlaying,
        zoom, setZoom,
        setTool,
        setFlip
    } = useItemViewContext();
    
    let overlayRef: HTMLDivElement | undefined;
    let previousFocus: HTMLElement | null = null;

    onMount(() => {
        previousFocus = document.activeElement as HTMLElement;
        // Focus the overlay to trap keyboard events and prevent background scrolling
        requestAnimationFrame(() => {
            overlayRef?.focus();
        });
    });

    onCleanup(() => {
        // Restore focus to the grid/list item
        previousFocus?.focus();
    });
    
    // Push image-viewer scope with blocking enabled (isolates input)
    createConditionalScope('image-viewer', () => true, undefined, true);
    
    const item = createMemo(() => lib.items.find(i => i.id.toString() === viewport.activeItemId()));

    // Reset view state when item changes
    createEffect(() => {
        const i = item();
        if (i) {
            reset();
            const type = getMediaType(i.filename);
            setMediaType(type);
        }
    });

    const navigate = (direction: number) => {
        const items = lib.items;
        const currentId = viewport.activeItemId();
        const currentIndex = items.findIndex(i => i.id.toString() === currentId);
        
        if (currentIndex !== -1) {
            const nextIndex = (currentIndex + direction + items.length) % items.length;
            viewport.openItem(items[nextIndex].id.toString());
        }
    };

    // Slideshow Timer Logic
    createEffect(() => {
        if (slideshowPlaying() && item()) {
            const durationMs = slideshowDuration() * 1000;
            const interval = setInterval(() => {
                navigate(1);
            }, durationMs);

            onCleanup(() => clearInterval(interval));
        }
    });
    
    // Stop slideshow on unmount (closing viewer)
    onCleanup(() => {
        setSlideshowPlaying(false);
    });

    const zoomIn = () => setZoom(Math.min(zoom() + 10, 500));
    const zoomOut = () => setZoom(Math.max(zoom() - 10, 5));
    const fitToScreen = () => window.dispatchEvent(new CustomEvent('viewport:fit'));
    const originalSize = () => setZoom(100);
    const toggleFlipH = () => setFlip((f: FlipState) => ({ ...f, horizontal: !f.horizontal }));
    const toggleFlipV = () => setFlip((f: FlipState) => ({ ...f, vertical: !f.vertical }));

    // Global navigation shortcuts (ItemView level)
    useShortcuts([
        { keys: 'Escape', name: 'Close Viewer', scope: 'image-viewer', action: () => viewport.closeItem() },
        { keys: 'Equal', name: 'Zoom In', scope: 'image-viewer', action: zoomIn },
        { keys: 'Minus', name: 'Zoom Out', scope: 'image-viewer', action: zoomOut },
        { keys: 'Meta+Digit0', name: 'Fit to Screen', scope: 'image-viewer', action: fitToScreen },
        { keys: 'Meta+Digit1', name: 'Original Size', scope: 'image-viewer', action: originalSize },
        { keys: 'KeyH', name: 'Pan Tool', scope: 'image-viewer', action: () => setTool("pan") },
        { keys: 'KeyR', name: 'Rotate Tool', scope: 'image-viewer', action: () => setTool("rotate") },
        { keys: 'ArrowLeft', name: 'Previous Item', scope: 'image-viewer', action: () => navigate(-1) },
        { keys: 'ArrowRight', name: 'Next Item', scope: 'image-viewer', action: () => navigate(1) },
        { keys: 'Space', name: 'Play/Pause Slideshow', scope: 'image-viewer', action: () => setSlideshowPlaying(!slideshowPlaying()) },
        { keys: 'Shift+KeyH', name: 'Flip Horizontal', scope: 'image-viewer', action: toggleFlipH },
        { keys: 'Shift+KeyV', name: 'Flip Vertical', scope: 'image-viewer', action: toggleFlipV },
    ]);

    return (
        <div 
            ref={overlayRef} 
            class="item-view-overlay"
            tabIndex={-1}
            style={{ outline: "none" }}
            role="dialog"
            aria-modal="true"
        >
            <BaseToolbar>
                <Switch>
                    <Match when={mediaType() === 'image' || mediaType() === 'unknown' || mediaType() === 'model'}>
                        <ImageToolbar />
                    </Match>
                    <Match when={mediaType() === 'font'}>
                        <FontToolbar />
                    </Match>
                </Switch>
            </BaseToolbar>
            
            <Show when={item()} fallback={<div class="item-error">Asset not found</div>}>
                <Switch fallback={<div class="item-error">Unsupported format</div>}>
                    <Match when={getMediaType(item()!.filename) === 'image' || getMediaType(item()!.filename) === 'unknown'}>
                       {/* Defaulting unknown to image for now, or maybe project/raw files */}
                        <ImageViewer 
                            src={`orig://localhost/${item()!.path}`} 
                            alt={item()!.filename}
                        />
                    </Match>
                    <Match when={getMediaType(item()!.filename) === 'video'}>
                        <VideoPlayer 
                            src={`orig://localhost/${item()!.path}`} 
                            type="video"
                        />
                    </Match>
                    <Match when={getMediaType(item()!.filename) === 'audio'}>
                        <VideoPlayer 
                            src={`orig://localhost/${item()!.path}`} 
                            type="audio"
                        />
                    </Match>
                    <Match when={mediaType() === 'font'}>
                        <FontView 
                            src={`orig://localhost/${item()!.path}`} 
                            fontName={item()!.filename}
                        />
                    </Match>
                    <Match when={getMediaType(item()!.filename) === 'model'}>
                        <ModelViewer 
                            src={`orig://localhost/${item()!.path}`} 
                            filename={item()!.filename}
                            thumbnail={item()!.thumbnail_path}
                        />
                    </Match>
                </Switch>
            </Show>
        </div>
    );
};
