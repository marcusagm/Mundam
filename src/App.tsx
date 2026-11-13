import { onMount, Show, createEffect, createSignal, Switch, Match } from 'solid-js';
import { useSystem, useFilters, useMetadataNotifications } from './core/hooks';
import { useIndexingNotifications } from './core/hooks/useIndexingNotifications';
import { useShutdownMedia } from './core/shutdown/useShutdownMedia';
import { WelcomeView, HomeView, GalleryView, DuplicateFinderView } from './views';
import { TitleBar, type ApplicationView } from './components/layout/TitleBar';
// Dialog removed (moved to WelcomeView)
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
import { InputProvider, useCommands } from './core/input';
// Removed logos (moved to WelcomeView)

/**
 * Main application component.
 * Manages global initialization, shortcuts, and the top-level layout.
 *
 * @returns {JSX.Element} The application layout.
 */
function App() {
    useShutdownMedia();
    const system = useSystem();
    const filters = useFilters();

    const [activeApplicationView, setActiveApplicationView] =
        createSignal<ApplicationView>('gallery');

    // Start background sync notifications
    useMetadataNotifications();
    useIndexingNotifications(system.indexingOutcome);

    useCommands({
        'app:settings': () => system.openSettings(true),
        'app:history-back': () => filters.goBack(),
        'app:history-forward': () => filters.goForward()
    });

    // Root-level DND cleanup
    createEffect(() => {
        if (!currentDragItem()) {
            setDropTargetId(null);
        }
    });

    onMount(() => {
        // Register Strategies
        dndRegistry.register('TAG', TagDropStrategy);
        dndRegistry.register('ASSET', AssetDropStrategy);
    });

    const renderTitleBar = () => (
        <TitleBar activeView={activeApplicationView()} onViewChange={setActiveApplicationView} />
    );

    return (
        <>
            <Show
                when={!system.loading()}
                fallback={<Loader isFullscreen text="Initializing Mundam..." />}
            >
                <Show when={system.rootPath()} fallback={<WelcomeView header={renderTitleBar()} />}>
                    <Switch fallback={<GalleryView header={renderTitleBar()} />}>
                        <Match when={activeApplicationView() === 'home'}>
                            <HomeView
                                header={renderTitleBar()}
                                onNavigate={setActiveApplicationView}
                            />
                        </Match>
                        <Match when={activeApplicationView() === 'gallery'}>
                            <GalleryView header={renderTitleBar()} />
                        </Match>
                        <Match when={activeApplicationView() === 'duplicates'}>
                            <DuplicateFinderView header={renderTitleBar()} />
                        </Match>
                    </Switch>

                    <SettingsModal
                        isOpen={system.isSettingsOpen()}
                        onClose={() => system.openSettings(false)}
                        initialTab="general"
                    />
                </Show>
            </Show>
            <Sonner position="bottom-right" useRichColors />
        </>
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
