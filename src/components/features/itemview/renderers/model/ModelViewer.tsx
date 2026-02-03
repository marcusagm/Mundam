import { Component, createMemo, Show, createEffect } from "solid-js";
import "@google/model-viewer";
import { Loader } from "../../../../ui/Loader";
import { useItemViewContext } from "../../ItemViewContext";
import "./model-viewer.css";

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
    const { modelSettings, resetTrigger } = useItemViewContext();
    let viewerRef: any;

    // Handle Reset
    createEffect(() => {
        // Track the trigger
        void resetTrigger();
        
        // Don't run on first mount (0) if we want, but actually it's fine.
        if (viewerRef) {
            // Reset Camera
            viewerRef.cameraOrbit = "auto auto auto";
            viewerRef.fieldOfView = "auto";
            viewerRef.jumpCameraToGoal();
        }
    });

    // Handle Auto Rotate specifically
    // Sometimes model-viewer pauses auto-rotate after interaction.
    // Changing the prop might not restart it if it's "paused".
    // We can force it by resetting interaction prompt or simply setting the property directly.
    createEffect(() => {
        if (viewerRef) {
            if (modelSettings().autoRotate) {
                viewerRef.autoRotate = true;
                viewerRef.play(); // Force play just in case
            } else {
                viewerRef.autoRotate = false;
                viewerRef.pause(); // Optional
            }
        }
    });

    // Determine the path to the cached GLB content
    const glbUrl = createMemo(() => {
        // ... (same logic)
        if (!props.thumbnail) return null;
        const filename = props.thumbnail.split(/[\\/]/).pop();
        if (!filename) return null;
        const stem = filename.replace(/\.[^/.]+$/, "");
        return `thumb://localhost/${stem}.glb`;
    });

    return (
        <div 
            class="model-viewer-container" 
            style={{ "background-color": modelSettings().backgroundColor }}
        >
            <Show when={glbUrl()} fallback={
                <div class="model-placeholder">
                     <span class="model-icon">🧊</span>
                     <p>Preview pending or unavailable</p>
                </div>
            }>
                {/* @ts-ignore */}
                <model-viewer
                    ref={viewerRef}
                    src={glbUrl()!}
                    poster={props.thumbnail ? `thumb://localhost/${props.thumbnail}` : undefined}
                    alt={`3D model: ${props.filename}`}
                    shadow-intensity={modelSettings().backgroundColor === '#111111' ? "1" : "0.5"}
                    camera-controls
                    ar-status="not-presenting"
                    interaction-prompt="none"
                    style={{ 
                        width: "100%", 
                        height: "100%", 
                        "--poster-color": "transparent",
                        "--progress-bar-color": "var(--accent-color, #00a0a9)", 
                        "--progress-bar-height": "4px",
                        "background-image": modelSettings().showGrid 
                            ? "linear-gradient(#333 1px, transparent 1px), linear-gradient(90deg, #333 1px, transparent 1px)" 
                            : "none",
                        "background-size": "50px 50px",
                        "background-position": "center center"
                    }}
                >
                    <div slot="poster" class="model-loading-overlay">
                        <Loader size="lg" text="Loading 3D Scene..." />
                    </div>
                </model-viewer>
            </Show>
        </div>
    );
};
