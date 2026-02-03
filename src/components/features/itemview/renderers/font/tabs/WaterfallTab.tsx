import { Component, For } from "solid-js";
import { useItemViewContext } from "../../../ItemViewContext";

const SIZES = [72, 60, 48, 36, 24, 18, 14, 12];

export const WaterfallTab: Component<{ fontFamily: string }> = (props) => {
    const { fontSettings } = useItemViewContext();

    return (
        <div class="font-waterfall-list">
            <For each={SIZES}>
                {(size) => (
                    <div class="font-waterfall-item">
                        <div class="font-waterfall-label">
                            {size}px
                        </div>
                        <div 
                            style={{
                                "font-family": `"${props.fontFamily}"`,
                                "font-size": `${size}px`,
                                "line-height": fontSettings().lineHeight,
                                "letter-spacing": `${fontSettings().letterSpacing}px`,
                                "font-weight": fontSettings().fontWeight,
                            }}
                            class="font-waterfall-text"
                        >
                            The quick brown fox jumps over the lazy dog.
                        </div>
                    </div>
                )}
            </For>
        </div>
    );
};
