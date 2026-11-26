import { Component, JSX, Show } from 'solid-js';
import { useViewport } from '../../core/hooks';
import { ListView } from '../features/viewport/layouts/ListView';
import { ItemView } from '../features/itemview/ItemView';
import { PluginViewportActions } from '../features/viewport/toolbar/PluginViewportActions';
import type { PluginInspectorSection } from '../../lib/pluginInspector';
import './viewport.css';

/** Composes the gallery and full asset views. */
export const Viewport: Component<{
    pluginSections: PluginInspectorSection[] | null | undefined;
    onRevealPluginSection: (sectionKey: string) => void;
}> = (properties): JSX.Element => {
    /**
     * Viewport store
     */
    const viewport = useViewport();

    return (
        <main class="viewport-root">
            <ListView
                toolbarActions={
                    <Show when={viewport.mode() !== 'item'}>
                        <PluginViewportActions
                            sections={properties.pluginSections}
                            onRevealSection={properties.onRevealPluginSection}
                        />
                    </Show>
                }
            />

            <Show when={viewport.mode() === 'item'}>
                <ItemView />
            </Show>
        </main>
    );
};
