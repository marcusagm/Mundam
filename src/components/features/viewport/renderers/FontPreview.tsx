import { Component, createSignal, onCleanup, onMount } from "solid-js";
import { useViewportContext } from "../ViewportContext";
import "./renderers.css";

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
        <div class="font-preview-container">
            <div class="font-controls">
                <span>{currentFontSize()}px ({Math.round(zoom())}%)</span>
            </div>

            {error() ? (
                <div class="font-error">{error()}</div>
            ) : !loaded() ? (
                <div class="font-loading">Loading font...</div>
            ) : (
                <textarea 
                    class="font-textarea"
                    style={{
                        "font-family": fontFamily(),
                        "font-size": `${currentFontSize()}px`,
                    }}
                    value={text()}
                    onInput={(e) => setText(e.currentTarget.value)}
                />
            )}
        </div>
    );
};
