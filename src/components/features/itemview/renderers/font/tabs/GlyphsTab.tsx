import { Component, createMemo, For } from "solid-js";
import { useItemViewContext } from "../../../ItemViewContext";

export const GlyphsTab: Component<{ fontFamily: string }> = (props) => {
    const { fontSettings } = useItemViewContext();

    const glyphs = createMemo(() => {
        const chars = [];
        // Basic Latin + Latin-1 Supplement + Generic Currency/Punctuation
        // 33-126 (Printable ASCII)
        // 161-255 (Latin-1)
        for (let i = 33; i <= 126; i++) chars.push(String.fromCharCode(i));
        for (let i = 161; i <= 255; i++) chars.push(String.fromCharCode(i));
        return chars;
    });

    return (
        <div class="font-glyphs-grid">
            <For each={glyphs()}>
                {(char) => (
                    <div class="font-glyph-item">
                        <div 
                            style={{
                                "font-family": `"${props.fontFamily}"`,
                                "font-size": `${fontSettings().fontSize}px`, 
                                // Use set size, or hardcode a reasonable size for grid?
                                // User probably wants to see detail. Let's use fontSize but clamp it if too big for box?
                                // Actually, let's use a fixed large size relative to box, or let user control it.
                                // Let's use fontSettings.fontSize but separate it?
                                // For now, respect toolbar.
                                "line-height": 1,
                                "font-weight": fontSettings().fontWeight,
                            }}
                        >
                            {char}
                        </div>
                        <div class="font-glyph-code">
                            {char.charCodeAt(0)}
                        </div>
                    </div>
                )}
            </For>
        </div>
    );
};
