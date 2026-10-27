/**
 * useNavigationShortcuts Primitive
 * Registers one "go to view" shortcut per navigation item (Mod+1, Mod+2, ...).
 */

import { createEffect, onCleanup, untrack, type Accessor } from 'solid-js';
import { shortcutStore } from '../store/shortcutStore';
import { onCommand } from '../dispatcher';

export interface NavigationShortcutItem<ViewId extends string = string> {
    /** Identifier handed back to `onNavigate` */
    id: ViewId;
    /** Human-readable name, shown in the settings panel as "Go to <label>" */
    label: string;
}

const MAX_NAVIGATION_SHORTCUTS = 9;

export function getNavigationCommand(viewId: string): string {
    return `navigate:${viewId}`;
}

/**
 * Keeps one customizable shortcut per navigation item in sync with `items`, so views
 * added later (for example by plugins) get their shortcut and settings entry automatically.
 * The first nine items are bound to Cmd/Ctrl+1 to Cmd/Ctrl+9 by default.
 *
 * @example
 * useNavigationShortcuts(
 *   () => [{ id: 'home', label: 'Home' }],
 *   viewId => setActiveView(viewId)
 * );
 */
export function useNavigationShortcuts<ViewId extends string>(
    items: Accessor<NavigationShortcutItem<ViewId>[]>,
    onNavigate: (viewId: ViewId) => void
): void {
    createEffect(() => {
        const navigationItems = items().slice(0, MAX_NAVIGATION_SHORTCUTS);
        const registeredIds: string[] = [];
        const unsubscribers: (() => void)[] = [];

        untrack(() => {
            navigationItems.forEach((item, index) => {
                const command = getNavigationCommand(item.id);

                registeredIds.push(
                    shortcutStore.register({
                        name: `Go to ${item.label}`,
                        description: `Switch to the ${item.label} view`,
                        keys: `Mod+Digit${index + 1}`,
                        scope: 'global',
                        command,
                        category: 'Navigation',
                        ignoreInputs: false
                    })
                );
                unsubscribers.push(onCommand(command, () => onNavigate(item.id)));
            });
        });

        onCleanup(() => {
            registeredIds.forEach(id => shortcutStore.unregister(id));
            unsubscribers.forEach(unsubscribe => unsubscribe());
        });
    });
}
