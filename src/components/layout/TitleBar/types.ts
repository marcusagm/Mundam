import { type Component } from 'solid-js';

/**
 * Represents the distinct views or sections of the application
 * that can be navigated to from the title bar.
 */
export type ApplicationView = 'home' | 'gallery' | 'duplicates';

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
    view: ApplicationView;

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
