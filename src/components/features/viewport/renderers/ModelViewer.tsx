import { Component } from "solid-js";

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
        <div class="model-viewer-placeholder" style={{
            "display": "flex",
            "flex-direction": "column",
            "align-items": "center",
            "justify-content": "center",
            "height": "100%",
            "color": "var(--text-muted)"
        }}>
            <div style={{ "font-size": "3rem", "margin-bottom": "1rem" }}>🧊</div>
            <h2>3D Model Preview</h2>
            <p>{props.filename}</p>
            <p style={{ "margin-top": "1rem", "opacity": 0.7 }}>
                Interaction for .blend/fbx/obj files requires a specialized 3D renderer.
            </p>
        </div>
    );
};
