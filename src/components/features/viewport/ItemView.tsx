import { Component, createMemo, Show, Switch, Match, createEffect } from "solid-js";
import { useViewport, useLibrary } from "../../../core/hooks";
import { ItemViewToolbar } from "./ItemViewToolbar";
import { createInputScope, useShortcuts } from "../../../core/input";
import { ViewportProvider, useViewportContext } from "./ViewportContext";
import { ImageViewer } from "./renderers/ImageViewer";
import { VideoPlayer } from "./renderers/VideoPlayer";
import { FontPreview } from "./renderers/FontPreview";
import { ModelViewer } from "./renderers/ModelViewer";
import "./item-view.css";

// Helper to determine media type from extension
const getMediaType = (filename: string): 'image' | 'video' | 'audio' | 'font' | 'model' | 'unknown' => {
    const ext = filename.split('.').pop()?.toLowerCase();
    if (!ext) return 'unknown';

    const imageExts = ['jpg', 'jpeg', 'jpe', 'jfif', 'png', 'webp', 'gif', 'bmp', 'ico', 'svg', 'avif']; 
    // Browser supported images. For others (psd, tif), we might need a different strategy, 
    // but for now sticking to the plan of "implementations for formats". 
    // If it's a RAW/PSD, ImageViewer will try to load it.

    const videoExts = ['mp4', 'm4v', 'webm', 'mov', 'qt', 'mxf', 'mkv'];
    const audioExts = ['mp3', 'wav', 'ogg', 'aac', 'flac', 'm4a'];
    const fontExts = ['ttf', 'otf', 'woff', 'woff2'];
    const modelExts = ['blend', 'fbx', 'obj', 'glb', 'gltf', 'stl'];

    if (imageExts.includes(ext)) return 'image';
    if (videoExts.includes(ext)) return 'video';
    if (audioExts.includes(ext)) return 'audio';
    if (fontExts.includes(ext)) return 'font';
    if (modelExts.includes(ext)) return 'model';
    
    return 'unknown';
};

export const ItemView: Component = () => {
    return (
        <ViewportProvider>
            <ItemViewContent />
        </ViewportProvider>
    );
};

const ItemViewContent: Component = () => {
    const viewport = useViewport();
    const lib = useLibrary();
    const { reset, setMediaType } = useViewportContext();
    
    // Push image-viewer scope
    createInputScope('image-viewer', undefined, true);
    
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

    // Global navigation shortcuts (ItemView level)
    useShortcuts([
        { keys: 'Escape', name: 'Close Viewer', scope: 'image-viewer', action: () => viewport.closeItem() },
        { keys: 'ArrowLeft', name: 'Previous Item', scope: 'image-viewer', action: () => navigate(-1) },
        { keys: 'ArrowRight', name: 'Next Item', scope: 'image-viewer', action: () => navigate(1) },
    ]);

    return (
        <div class="item-view-overlay">
            <ItemViewToolbar />
            
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
                    <Match when={getMediaType(item()!.filename) === 'font'}>
                        <FontPreview 
                            src={`orig://localhost/${item()!.path}`} 
                            fontName={item()!.filename}
                        />
                    </Match>
                    <Match when={getMediaType(item()!.filename) === 'model'}>
                        <ModelViewer 
                            src={`orig://localhost/${item()!.path}`} 
                            filename={item()!.filename}
                        />
                    </Match>
                </Switch>
            </Show>
        </div>
    );
};
