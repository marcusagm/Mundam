import { createEffect, createMemo, createSignal, type Accessor } from 'solid-js';
import { pluginScreenRoute, type PluginPresentation } from '../lib/pluginPresentation';
import type { ApplicationView } from '../components/layout/TitleBar/types';

const EMPTY_SCREENS: PluginPresentation['screens'] = [];

/** Routes only to approved plugin screens and restores the gallery when the active screen is lost. */
export function createApplicationNavigation(
    presentation: Accessor<PluginPresentation | null | undefined>,
    catalogAvailable: Accessor<boolean>
) {
    const [activeView, setActiveView] = createSignal<ApplicationView>('gallery');
    const [galleryFocusPending, setGalleryFocusPending] = createSignal(false);
    const screens = () => presentation()?.screens ?? EMPTY_SCREENS;
    const activeScreen = createMemo(() =>
        screens().find(screen => pluginScreenRoute(screen) === activeView())
    );
    const showingPluginScreen = () => activeView().startsWith('plugin:');

    const navigate = (view: ApplicationView) => {
        if (
            view.startsWith('plugin:') &&
            !screens().some(screen => pluginScreenRoute(screen) === view)
        ) {
            return;
        }
        setGalleryFocusPending(view === 'gallery' && showingPluginScreen());
        setActiveView(view);
    };

    createEffect(() => {
        if (
            showingPluginScreen() &&
            (!catalogAvailable() || (presentation() !== undefined && !activeScreen()))
        ) {
            navigate('gallery');
        }
    });

    return {
        activeView,
        activeScreen,
        showingPluginScreen,
        screens,
        navigate,
        galleryFocusPending,
        acknowledgeGalleryFocus: () => setGalleryFocusPending(false)
    };
}
