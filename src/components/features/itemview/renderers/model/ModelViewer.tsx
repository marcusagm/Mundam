import { Component, createMemo, Show } from "solid-js";
import "@google/model-viewer";
import "../renderers.css";

declare module "solid-js" {
    namespace JSX {
        interface IntrinsicElements {
            "model-viewer": any;
        }
    }
}

interface ModelViewerProps {
    src: string;
    filename: string;
    thumbnail?: string | null;
}

export const ModelViewer: Component<ModelViewerProps> = (props) => {
    // Determine the path to the cached GLB content
    const glbUrl = createMemo(() => {
        // Strategy: We rely on the backend pipeline which generates a .glb 
        // with the same hash name as the thumbnail (.webp).
        if (!props.thumbnail) return null;
        
        const filename = props.thumbnail.split(/[\\/]/).pop();
        if (!filename) return null;
        
        const stem = filename.replace(/\.[^/.]+$/, "");
        return `thumb://localhost/${stem}.glb`;
    });

    return (
        <div class="model-viewer-container">
            <Show when={glbUrl()} fallback={
                <div class="model-placeholder">
                     <div class="model-icon">🧊</div>
                     <p>Preview pending or unavailable</p>
                </div>
            }>
                {/* @ts-ignore */}
                <model-viewer
                    src={glbUrl()!}
                    poster={props.thumbnail ? `thumb://localhost/${props.thumbnail}` : undefined}
                    alt={`3D model: ${props.filename}`}
                    shadow-intensity="1"
                    camera-controls
                    auto-rotate
                    ar-status="not-presenting"
                    style={{ 
                        width: "100%", 
                        height: "100%", 
                        "--poster-color": "transparent",
                        "--progress-bar-color": "#eab308", 
                        "--progress-bar-height": "4px"
                    }}
                >
                    <div slot="poster" class="model-loading-overlay">
                        <div class="model-loader-spinner"></div>
                        <span>Loading 3D Scene...</span>
                    </div>
                </model-viewer>
            </Show>
        </div>
    );
};
