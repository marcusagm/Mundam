import { Component, createSignal, onCleanup, onMount, Switch, Match, Show } from "solid-js";
import { useItemViewContext } from "../../ItemViewContext";
import { Button } from "../../../../ui/Button";
import { PreviewTab } from "./tabs/PreviewTab";
import { WaterfallTab } from "./tabs/WaterfallTab";
import { GlyphsTab } from "./tabs/GlyphsTab";
import { InfoTab } from "./tabs/InfoTab";
import "../renderers.css";
import "./font-view.css";

interface FontViewProps {
    src: string;
    fontName: string; 
}

export const FontView: Component<FontViewProps> = (props) => {
    const { fontSettings } = useItemViewContext(); // Removed unused zoom
    const [loaded, setLoaded] = createSignal(false);
    const [error, setError] = createSignal<string | null>(null);
    const [activeTab, setActiveTab] = createSignal<'preview' | 'waterfall' | 'glyphs' | 'info'>('preview');
    const [fontFamily, setFontFamily] = createSignal<string>('sans-serif');

    onMount(async () => {
        try {
            const familyName = `font-preview-${props.fontName.replace(/\s+/g, '-')}-${Date.now()}`;
            // Load Font
            const fontFace = new FontFace(familyName, `url(${props.src})`);
            await fontFace.load();
            document.fonts.add(fontFace);
            
            setFontFamily(familyName);
            setLoaded(true);

            onCleanup(() => {
                document.fonts.delete(fontFace);
            });
        } catch (err) {
            console.error(err);
            setError("Failed to load font file.");
        }
    });

    return (
        <div class="font-view-container">
            {/* Tabs Header */}
            <div class="font-tabs">
                <Button 
                    variant={activeTab() === 'preview' ? "outline" : "ghost"} 
                    size="sm" 
                    onClick={() => setActiveTab('preview')}
                >
                    Preview
                </Button>
                <Button 
                    variant={activeTab() === 'waterfall' ? "outline" : "ghost"} 
                    size="sm" 
                    onClick={() => setActiveTab('waterfall')}
                >
                    Waterfall
                </Button>
                <Button 
                    variant={activeTab() === 'glyphs' ? "outline" : "ghost"} 
                    size="sm" 
                    onClick={() => setActiveTab('glyphs')}
                >
                    Glyphs
                </Button>
                <Button 
                    variant={activeTab() === 'info' ? "outline" : "ghost"} 
                    size="sm" 
                    onClick={() => setActiveTab('info')}
                >
                    Info
                </Button>
            </div>

            <Show when={!error()} fallback={<div class="font-error">{error()}</div>}>
                <Show when={loaded()} fallback={<div class="font-loading">Loading font...</div>}>
                    <div 
                        class="font-content"
                        style={{
                            "background-color": fontSettings().backgroundColor,
                            "color": fontSettings().color,
                        }}
                    >
                         <div class="font-inner-wrapper">
                            <Switch>
                                <Match when={activeTab() === 'preview'}>
                                    <PreviewTab fontFamily={fontFamily()} />
                                </Match>
                                <Match when={activeTab() === 'waterfall'}>
                                    <WaterfallTab fontFamily={fontFamily()} />
                                </Match>
                                <Match when={activeTab() === 'glyphs'}>
                                    <GlyphsTab fontFamily={fontFamily()} />
                                </Match>
                                <Match when={activeTab() === 'info'}>
                                    <InfoTab fontFamily={fontFamily()} src={props.src} name={props.fontName} />
                                </Match>
                            </Switch>
                        </div>
                    </div>
                </Show>
            </Show>
        </div>
    );
};
