import { onMount, onCleanup, Show, createEffect, createSignal, Switch, Match } from 'solid-js';
import {
    useSystem,
    useNotification,
    useSelection,
    useLibrary,
    useMetadataNotifications
} from './core/hooks';
import { WelcomeView, HomeView, GalleryView, DuplicateFinderView } from './views';
import { TitleBar, type ApplicationView } from './components/layout/TitleBar';
// Dialog removed (moved to WelcomeView)
import { listen } from '@tauri-apps/api/event';
// Native DnD
import {
    dndRegistry,
    TagDropStrategy,
    AssetDropStrategy,
    currentDragItem,
    setDropTargetId
} from './core/dnd';
import { Sonner } from './components/ui';
import { Loader } from './components/ui/Loader';
import { SettingsModal } from './components/features/settings';
// Input System
import { InputProvider, useShortcuts } from './core/input';
// Removed logos (moved to WelcomeView)

/**
 * Main application component.
 * Manages global initialization, shortcuts, and the top-level layout.
 *
 * @returns {JSX.Element} The application layout.
 */
function App() {
    const system = useSystem();
    const notification = useNotification();
    const selection = useSelection();
    const lib = useLibrary();

    const [activeApplicationView, setActiveApplicationView] =
        createSignal<ApplicationView>('gallery');

    // Start background sync notifications
    useMetadataNotifications();

    // Global shortcuts via Input Service
    useShortcuts([
        {
            keys: 'Meta+Comma',
            name: 'Settings',
            action: () => system.openSettings(true)
        },
        {
            keys: 'Meta+KeyA',
            name: 'Select All',
            ignoreInputs: true,
            action: () => {
                const allIds = lib.items.map(i => i.id);
                selection.select(allIds);
            }
        },
        {
            keys: 'Escape',
            name: 'Deselect All',
            action: () => {
                const active = document.activeElement;
                if (active && ['INPUT', 'TEXTAREA'].includes(active.tagName)) {
                    (active as HTMLElement).blur();
                } else {
                    selection.select([]);
                }
            }
        }
    ]);

    // Root-level DND cleanup
    createEffect(() => {
        if (!currentDragItem()) {
            setDropTargetId(null);
        }
    });

    onMount(() => {
        system.initialize();
        import('./core/store/appearanceStore').then(({ appearanceActions }) => {
            appearanceActions.initialize();
        });
        import('./core/store/formatStore').then(({ formatActions }) => {
            formatActions.initialize();
        });

        // Register Strategies
        dndRegistry.register('TAG', TagDropStrategy);
        dndRegistry.register('ASSET', AssetDropStrategy);

        // Listen for indexing completion (with proper cleanup)
        let unlistenIndexerComplete: (() => void) | null = null;
        listen('indexer:complete', () => {
            notification.success('Indexing Complete', 'Library update finished');
        }).then(unlisten => {
            unlistenIndexerComplete = unlisten;
        });

        onCleanup(() => {
            if (unlistenIndexerComplete) unlistenIndexerComplete();
        });

        // Notify Splash Screen
        window.dispatchEvent(new CustomEvent('app-ready'));
    });

    const renderTitleBar = () => (
        <TitleBar activeView={activeApplicationView()} onViewChange={setActiveApplicationView} />
    );

    return (
        <Show
            when={!system.loading()}
            fallback={<Loader isFullscreen text="Initializing Mundam..." />}
        >
            <Show when={system.rootPath()} fallback={<WelcomeView header={renderTitleBar()} />}>
                <Switch fallback={<GalleryView header={renderTitleBar()} />}>
                    <Match when={activeApplicationView() === 'home'}>
                        <HomeView header={renderTitleBar()} onNavigate={setActiveApplicationView} />
                    </Match>
                    <Match when={activeApplicationView() === 'gallery'}>
                        <GalleryView header={renderTitleBar()} />
                    </Match>
                    <Match when={activeApplicationView() === 'duplicates'}>
                        <DuplicateFinderView header={renderTitleBar()} />
                    </Match>
                </Switch>

                <Sonner position="bottom-right" useRichColors />
                <SettingsModal
                    isOpen={system.isSettingsOpen()}
                    onClose={() => system.openSettings(false)}
                    initialTab="general"
                />
            </Show>
        </Show>
    );
}

/**
 * Root application wrapper with context providers.
 *
 * @returns {JSX.Element} The application with providers.
 */
function AppWithProvider() {
    return (
        <InputProvider>
            <App />
        </InputProvider>
    );
}

export default AppWithProvider;
