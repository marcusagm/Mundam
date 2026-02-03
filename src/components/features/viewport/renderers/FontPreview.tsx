import { Component, createSignal, onCleanup, onMount } from "solid-js";
import { useViewportContext } from "../ViewportContext";

interface FontPreviewProps {
    src: string;
    fontName: string; // Unique identifier for the font face
}

export const FontPreview: Component<FontPreviewProps> = (props) => {
    const { zoom } = useViewportContext();
    const [loaded, setLoaded] = createSignal(false);
    const [error, setError] = createSignal<string | null>(null);
    const [text, setText] = createSignal("The quick brown fox jumps over the lazy dog.\n0123456789");
    
    // Base font size at 100% zoom
    const BASE_SIZE = 48; // px

    onMount(async () => {
        try {
            // Clean up name to be CSS safe
            const familyName = `font-preview-${props.fontName.replace(/\s+/g, '-')}-${Date.now()}`;
            
            // Create FontFace
            const fontFace = new FontFace(familyName, `url(${props.src})`);
            
            await fontFace.load();
            document.fonts.add(fontFace);
            
            // Set the family name to state to use in style
            (window as any)._currentFontFamily = familyName; 
            
            setLoaded(true);

            onCleanup(() => {
                document.fonts.delete(fontFace);
            });
        } catch (err) {
            console.error(err);
            setError("Failed to load font file.");
        }
    });

    const fontFamily = () => (window as any)._currentFontFamily || 'sans-serif';
    const currentFontSize = () => Math.round(BASE_SIZE * (zoom() / 100));

    return (
        <div class="font-preview-container" style={{
            "width": "100%",
            "height": "100%",
            "display": "flex",
            "flex-direction": "column",
            "padding": "2rem",
            "overflow": "hidden",
            "background": "var(--bg-secondary, #1a1a1a)",
            "color": "var(--text-primary, #fff)"
        }}>
            <div class="font-controls" style={{ "margin-bottom": "1rem", "display": "flex", "gap": "1rem" }}>
                <span>{currentFontSize()}px ({Math.round(zoom())}%)</span>
            </div>

            {error() ? (
                <div class="error">{error()}</div>
            ) : !loaded() ? (
                <div class="loading">Loading font...</div>
            ) : (
                <textarea 
                    style={{
                        "font-family": fontFamily(),
                        "font-size": `${currentFontSize()}px`,
                        "flex": 1,
                        "width": "100%",
                        "background": "transparent",
                        "border": "none",
                        "resize": "none",
                        "color": "currentColor",
                        "outline": "none"
                    }}
                    value={text()}
                    onInput={(e) => setText(e.currentTarget.value)}
                />
            )}
        </div>
    );
};
