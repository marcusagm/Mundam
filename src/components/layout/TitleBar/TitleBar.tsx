import { Component, For, Show } from 'solid-js';
import { House, GalleryVerticalEnd, FileStack } from 'lucide-solid';
import { Dynamic } from 'solid-js/web';
import { Tooltip } from '../../ui';
import { useMetadata } from '../../../core/hooks';
import { detectPlatform } from '../../../core/input/utils/platform';
import {
    getNavigationCommand,
    shortcutStore,
    useNavigationShortcuts,
    formatShortcutForDisplay
} from '../../../core/input';
import { formatCompactNumber } from '../../../utils/format';
import { WindowControls } from './WindowControls';
import type { TitleBarProperties, TitleBarNavigationItem, ApplicationView } from './types';
import './title-bar.css';

/**
 * Builds the list of navigation items displayed in the title bar.
 * Each item maps to a distinct application view.
 *
 * @returns {TitleBarNavigationItem[]} The navigation item definitions.
 */
function buildNavigationItems(): TitleBarNavigationItem[] {
    return [
        {
            view: 'home',
            label: 'Home',
            icon: () => <House />
        },
        {
            view: 'gallery',
            label: 'Asset Gallery',
            icon: () => <GalleryVerticalEnd />
        },
        {
            view: 'duplicates',
            label: 'Duplicate Finder',
            icon: () => <FileStack />
        }
    ];
}

/**
 * Custom title bar for the application window.
 * Replaces the native OS title bar with a custom one that includes:
 * - Platform-specific window controls (macOS traffic lights or Windows/Linux buttons)
 * - Navigation buttons for switching between application views
 * - A draggable region for moving the window
 *
 * On macOS, the native traffic lights are rendered by the OS via Tauri's
 * `titleBarStyle: "overlay"` config, so the component reserves an inset area for them.
 * On Windows/Linux, custom minimize/maximize/close buttons are rendered explicitly.
 *
 * @param {TitleBarProperties} properties - Component properties.
 * @returns {JSX.Element} The rendered title bar.
 */
export const TitleBar: Component<TitleBarProperties> = properties => {
    const platform = detectPlatform();
    const metadata = useMetadata();
    const navigationItems = buildNavigationItems();

    useNavigationShortcuts(
        () => navigationItems.map(item => ({ id: item.view, label: item.label })),
        view => properties.onViewChange(view)
    );

    const getTooltipContent = (item: TitleBarNavigationItem): string => {
        const shortcut = shortcutStore.getByCommand(getNavigationCommand(item.view));
        return shortcut
            ? `${item.label} (${formatShortcutForDisplay(shortcut.keys as string)})`
            : item.label;
    };

    /**
     * Handles a navigation button click, delegating the view change to the parent.
     *
     * @param {ApplicationView} targetView - The view to navigate to.
     * @returns {void}
     */
    const handleNavigationClick = (targetView: ApplicationView): void => {
        properties.onViewChange(targetView);
    };

    return (
        <header class="titlebar" classList={{ 'titlebar--macos': platform === 'mac' }}>
            {/*
             * macOS: reserve space on the left for the native traffic lights.
             * Windows/Linux: no left-side placeholder needed.
             */}
            <Show when={platform === 'mac'}>
                <div class="titlebar-platform-area" />
            </Show>

            {/* Navigation Buttons */}
            <nav class="titlebar-navigation" aria-label="Application views">
                <For each={navigationItems}>
                    {navigationItem => (
                        <Tooltip content={getTooltipContent(navigationItem)} placement="bottom">
                            <button
                                class="titlebar-navigation-button"
                                classList={{
                                    'titlebar-navigation-button--active':
                                        properties.activeView === navigationItem.view
                                }}
                                aria-label={navigationItem.label}
                                aria-current={
                                    properties.activeView === navigationItem.view
                                        ? 'page'
                                        : undefined
                                }
                                onClick={() => handleNavigationClick(navigationItem.view)}
                            >
                                <Dynamic component={navigationItem.icon} />
                                {navigationItem.view === 'duplicates' &&
                                    metadata.stats.duplicate_assets > 0 && (
                                        <span class="titlebar-navigation-badge">
                                            {formatCompactNumber(
                                                metadata.stats.duplicate_assets,
                                                99
                                            )}
                                        </span>
                                    )}
                            </button>
                        </Tooltip>
                    )}
                </For>
            </nav>

            {/* Draggable Region – expands to fill remaining space */}
            <div class="titlebar-drag-region" data-tauri-drag-region />

            {/*
             * Windows/Linux: custom minimize/maximize/close buttons on the far right,
             * matching the OS convention. Hidden on macOS (native traffic lights are used).
             */}
            <Show when={platform !== 'mac'}>
                <div class="titlebar-platform-area">
                    <WindowControls />
                </div>
            </Show>
        </header>
    );
};
