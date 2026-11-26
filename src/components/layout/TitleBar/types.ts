import { type Component, type JSX } from 'solid-js';
import type { PluginScreenRoute } from '../../../lib/pluginPresentation';

/**
 * Represents the distinct views or sections of the application
 * that can be navigated to from the title bar.
 */
export type NativeApplicationView = 'home' | 'gallery' | 'duplicates';
export type ApplicationView = NativeApplicationView | PluginScreenRoute;

/**
 * Properties for the TitleBar component.
 */
export interface TitleBarProperties {
    /**
     * The currently active application view, used to highlight the active navigation button.
     *
     * @type {ApplicationView}
     */
    activeView: ApplicationView;

    /** Hide view switching while the library is unconfigured. */
    hideNavigation?: boolean;

    /** Optional gallery actions rendered beside the window controls. */
    pluginActions?: JSX.Element;
    /** Optional navigation menu for approved application screens. */
    pluginScreens?: JSX.Element;

    /**
     * Callback invoked when the user clicks a navigation button to change views.
     *
     * @param {ApplicationView} view - The target view to navigate to.
     * @returns {void}
     */
    onViewChange: (view: ApplicationView) => void;
}

/**
 * Describes a single navigation item rendered in the title bar.
 */
export interface TitleBarNavigationItem {
    /**
     * The application view this item navigates to.
     *
     * @type {ApplicationView}
     */
    view: NativeApplicationView;

    /**
     * The user-facing label displayed as a tooltip.
     *
     * @type {string}
     */
    label: string;

    /**
     * The SVG icon element rendered inside the button.
     *
     * @type {Component}
     */
    icon: Component;
}
