import { Button } from '../../../../ui';
import { Component, createEffect, createSignal, onCleanup, Switch, Match, Show } from 'solid-js';
import { useItemViewContext } from '../../ItemViewContext';
import { PreviewTab } from './tabs/PreviewTab';
import { WaterfallTab } from './tabs/WaterfallTab';
import { GlyphsTab } from './tabs/GlyphsTab';
import { InfoTab } from './tabs/InfoTab';
import '../renderers.css';
import './font-view.css';

interface FontViewProps {
    src: string;
    fontName: string;
    /** Called when the font finished loading or failed */
    onReady?: () => void;
}

export const FontView: Component<FontViewProps> = props => {
    const { fontSettings } = useItemViewContext(); // Removed unused zoom
    const [loaded, setLoaded] = createSignal(false);
    const [error, setError] = createSignal<string | null>(null);
    const [activeTab, setActiveTab] = createSignal<'preview' | 'waterfall' | 'glyphs' | 'info'>(
        'preview'
    );
    const [fontFamily, setFontFamily] = createSignal<string>('sans-serif');

    createEffect(() => {
        const source = props.src;
        const fontName = props.fontName;
        let cancelled = false;
        let loadedFace: FontFace | undefined;

        setLoaded(false);
        setError(null);

        onCleanup(() => {
            cancelled = true;
            if (loadedFace) document.fonts.delete(loadedFace);
        });

        (async () => {
            try {
                const familyName = `font-preview-${fontName.replace(/\s+/g, '-')}-${Date.now()}`;
                const fontFace = new FontFace(familyName, `url(${source})`);
                await fontFace.load();
                if (cancelled) return;
                document.fonts.add(fontFace);
                loadedFace = fontFace;

                setFontFamily(familyName);
                setLoaded(true);
            } catch (err) {
                if (cancelled) return;
                console.error(err);
                setError('Failed to load font file.');
            }
            props.onReady?.();
        })();
    });

    return (
        <div class="font-view-container">
            {/* Tabs Header */}
            <div class="font-tabs">
                <Button
                    variant={activeTab() === 'preview' ? 'outline' : 'ghost'}
                    size="sm"
                    onClick={() => setActiveTab('preview')}
                >
                    Preview
                </Button>
                <Button
                    variant={activeTab() === 'waterfall' ? 'outline' : 'ghost'}
                    size="sm"
                    onClick={() => setActiveTab('waterfall')}
                >
                    Waterfall
                </Button>
                <Button
                    variant={activeTab() === 'glyphs' ? 'outline' : 'ghost'}
                    size="sm"
                    onClick={() => setActiveTab('glyphs')}
                >
                    Glyphs
                </Button>
                <Button
                    variant={activeTab() === 'info' ? 'outline' : 'ghost'}
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
                            'background-color': fontSettings().backgroundColor,
                            color: fontSettings().color
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
                                    <InfoTab
                                        fontFamily={fontFamily()}
                                        src={props.src}
                                        name={props.fontName}
                                    />
                                </Match>
                            </Switch>
                        </div>
                    </div>
                </Show>
            </Show>
        </div>
    );
};
