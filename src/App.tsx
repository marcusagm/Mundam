import { onMount, Show, createEffect, Switch, Match, type JSX } from 'solid-js';
import { useSystem, useFilters, useMetadataNotifications } from './core/hooks';
import { useIndexingNotifications } from './core/hooks/useIndexingNotifications';
import { useShutdownMedia } from './core/shutdown/useShutdownMedia';
import { WelcomeView, HomeView, GalleryView, DuplicateFinderView } from './views';
import { TitleBar } from './components/layout/TitleBar';
import { PluginScreensMenu } from './components/layout/TitleBar/PluginScreensMenu';
import { PluginScreenView } from './views/PluginScreenView';
import { createApplicationNavigation } from './views/createApplicationNavigation';
import { createPluginPresentation } from './core/hooks/createPluginPresentation';
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
import { desktopImportPreferences } from './core/tauri/importPreferences';
import { desktopExternalImport } from './core/tauri/externalImport';
import { desktopPluginPresentation, desktopPlugins } from './core/tauri/plugins';
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

    const activeCatalog = () => (system.loading() ? undefined : system.rootPath() || undefined);
    const pluginPresentation = createPluginPresentation(
        () => desktopPluginPresentation,
        activeCatalog
    );
    const navigation = createApplicationNavigation(pluginPresentation.snapshot, () =>
        Boolean(activeCatalog())
    );

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

    const renderTitleBar = (pluginActions?: JSX.Element) => (
        <TitleBar
            hideNavigation={!system.rootPath()}
            activeView={navigation.activeView()}
            onViewChange={navigation.navigate}
            pluginActions={pluginActions}
            pluginScreens={
                <PluginScreensMenu
                    screens={navigation.screens()}
                    onNavigate={navigation.navigate}
                />
            }
        />
    );

    return (
        <>
            <Show
                when={!system.loading()}
                fallback={<Loader isFullscreen text="Initializing Mundam..." />}
            >
                <Show when={system.rootPath()} fallback={<WelcomeView header={renderTitleBar()} />}>
                    <Switch
                        fallback={
                            <GalleryView
                                header={renderTitleBar}
                                pluginPresentation={pluginPresentation}
                                focusOnReturn={
                                    navigation.galleryFocusPending() && !system.isSettingsOpen()
                                }
                                onReturnFocused={navigation.acknowledgeGalleryFocus}
                            />
                        }
                    >
                        <Match when={navigation.activeView() === 'home'}>
                            <HomeView header={renderTitleBar()} onNavigate={navigation.navigate} />
                        </Match>
                        <Match when={navigation.activeView() === 'duplicates'}>
                            <DuplicateFinderView header={renderTitleBar()} />
                        </Match>
                        <Match when={navigation.showingPluginScreen()}>
                            <Show
                                when={navigation.activeScreen()}
                                keyed
                                fallback={<Loader isFullscreen text="Loading plugin screen..." />}
                            >
                                {screen => (
                                    <PluginScreenView
                                        header={renderTitleBar()}
                                        screen={screen}
                                        focusHeading={!system.isSettingsOpen()}
                                        onBack={() => navigation.navigate('gallery')}
                                    />
                                )}
                            </Show>
                        </Match>
                    </Switch>
                </Show>
                <SettingsModal
                    isOpen={system.isSettingsOpen()}
                    importPreferences={desktopImportPreferences}
                    externalImport={desktopExternalImport}
                    plugins={desktopPlugins}
                    onClose={() => system.openSettings(false)}
                    initialTab="general"
                />
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
