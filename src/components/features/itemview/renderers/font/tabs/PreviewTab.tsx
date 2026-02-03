import { Component, createSignal } from "solid-js";
import { useItemViewContext } from "../../../ItemViewContext";

export const PreviewTab: Component<{ fontFamily: string }> = (props) => {
    const { fontSettings } = useItemViewContext();
    const [text, setText] = createSignal("The quick brown fox jumps over the lazy dog.\n0123456789\nAvailable for your reading pleasure.");

    return (
        <div class="font-tab-preview">
            <textarea
                class="font-preview-textarea"
                style={{
                    "font-family": `"${props.fontFamily}"`,
                    "font-size": `${fontSettings().fontSize}px`,
                    "line-height": fontSettings().lineHeight,
                    "letter-spacing": `${fontSettings().letterSpacing}px`,
                    "font-weight": fontSettings().fontWeight,
                    "color": "inherit" // Inherits from FontView container
                }}
                value={text()}
                onInput={(e) => setText(e.currentTarget.value)}
                spellcheck={false}
            />
        </div>
    );
};
