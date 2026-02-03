import { Component } from "solid-js";
import "./renderers.css";

interface ModelViewerProps {
    src: string;
    filename: string;
}

export const ModelViewer: Component<ModelViewerProps> = (props) => {
    // Ideally we would use <model-viewer> or Three.js
    // Since we don't have dependencies, we will show a placeholder
    // suggesting what to do, or try to use a CDN based approach (not recommended for offline app)
    // or just a nice UI saying "3D Preview not available"

    return (
        <div class="model-viewer-placeholder">
            <div class="model-icon">🧊</div>
            <h2>3D Model Preview</h2>
            <p>{props.filename}</p>
            <p class="model-subtitle">
                Interaction for .blend/fbx/obj files requires a specialized 3D renderer.
            </p>
        </div>
    );
};
