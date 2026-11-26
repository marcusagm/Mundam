import { Component, createEffect, JSX, Show } from 'solid-js';
import { useViewport } from '../../core/hooks';
import { ListView } from '../features/viewport/layouts/ListView';
import { ItemView } from '../features/itemview/ItemView';
import { PluginSectionActionMenu } from '../features/inspector/plugins/PluginSectionActionMenu';
import type { PluginInspectorSection } from '../../lib/pluginInspector';
import './viewport.css';

/** Composes the gallery and full asset views. */
export const Viewport: Component<{
    pluginSections: PluginInspectorSection[] | null | undefined;
    onRevealPluginSection: (sectionKey: string) => void;
    focusOnReturn?: boolean;
    onReturnFocused?: () => void;
}> = (properties): JSX.Element => {
    /**
     * Viewport store
     */
    const viewport = useViewport();
    let gallery: HTMLElement | undefined;
    createEffect(() => {
        if (properties.focusOnReturn && gallery) {
            gallery.focus();
            properties.onReturnFocused?.();
        }
    });

    return (
        <main class="viewport-root" aria-label="Asset Gallery" tabIndex={-1} ref={gallery}>
            <ListView
                toolbarActions={
                    <Show when={viewport.mode() !== 'item'}>
                        <PluginSectionActionMenu
                            surface="viewport"
                            buttonLabel="Plugin actions"
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
