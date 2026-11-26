import { For, Show, type Component } from 'solid-js';
import { Dynamic } from 'solid-js/web';
import { ResizablePanelGroup, ResizablePanel, ResizableHandle } from '../ui/Resizable';
import { LibrarySidebarPanel } from '../features/library/LibrarySidebarPanel';
import { FolderTreeSidebarPanel } from '../features/library/FolderTreeSidebarPanel';
import { TagTreeSidebarPanel } from '../features/tags/TagTreeSidebarPanel';
import { SmartFoldersSidebarPanel } from '../features/search/SmartFoldersSidebarPanel';
import { PluginSidebarSections } from '../features/library/plugins/PluginSidebarSections';
import type { PluginInspectorSection } from '../../lib/pluginInspector';
import './library-sidebar.css';

const NATIVE_PANELS = [
    {
        id: 'sidebar-library',
        defaultSize: 28,
        minSize: 28,
        className: 'panel-lib',
        panel: LibrarySidebarPanel
    },
    {
        id: 'sidebar-folders',
        defaultSize: 22,
        minSize: 20,
        className: 'panel-folders',
        panel: FolderTreeSidebarPanel
    },
    {
        id: 'sidebar-tags',
        defaultSize: 20,
        minSize: 20,
        className: 'panel-tags',
        panel: TagTreeSidebarPanel
    },
    {
        id: 'sidebar-smart',
        defaultSize: 20,
        minSize: 20,
        className: 'panel-smart',
        panel: SmartFoldersSidebarPanel
    }
] as const;

const LAYOUT_STORAGE_KEY = 'sidebar-layout-v3';

function readPanelLayout(): Partial<Record<string, number>> {
    try {
        const saved: unknown = JSON.parse(localStorage.getItem(LAYOUT_STORAGE_KEY) ?? '{}');
        if (!saved || typeof saved !== 'object' || Array.isArray(saved)) return {};
        const sizes = saved as Record<string, unknown>;
        return Object.fromEntries(
            NATIVE_PANELS.flatMap(({ id }) => {
                const size = sizes[id];
                return typeof size === 'number' && size > 0 && size <= 100 ? [[id, size]] : [];
            })
        );
    } catch {
        return {};
    }
}

function savePanelLayout(sizes: number[]): void {
    if (sizes.length !== NATIVE_PANELS.length) return;
    const layout = Object.fromEntries(NATIVE_PANELS.map(({ id }, index) => [id, sizes[index]]));
    localStorage.setItem(LAYOUT_STORAGE_KEY, JSON.stringify(layout));
}

/** Renders the fixed library panels and the separate plugin navigation region. */
export const LibrarySidebar: Component<{
    pluginSections: PluginInspectorSection[] | null | undefined;
    canOpenPluginLinks: boolean;
    onOpenPluginSection: (sectionKey: string) => void;
}> = properties => {
    const persistedLayout = readPanelLayout();

    return (
        <aside class="library-sidebar">
            <ResizablePanelGroup
                direction="vertical"
                class="library-sidebar-native"
                onLayout={savePanelLayout}
            >
                <For each={NATIVE_PANELS}>
                    {(panel, index) => (
                        <>
                            <Show when={index() > 0}>
                                <ResizableHandle />
                            </Show>
                            <ResizablePanel
                                id={panel.id}
                                defaultSize={persistedLayout[panel.id] ?? panel.defaultSize}
                                minSize={panel.minSize}
                                class={panel.className}
                            >
                                <Dynamic component={panel.panel} />
                            </ResizablePanel>
                        </>
                    )}
                </For>
            </ResizablePanelGroup>
            <PluginSidebarSections
                sections={properties.pluginSections}
                canOpenLinks={properties.canOpenPluginLinks}
                onOpenSection={properties.onOpenPluginSection}
            />
        </aside>
    );
};
