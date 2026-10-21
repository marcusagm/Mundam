/**
 * Menu Navigation Hook
 *
 * Manages keyboard focus state and shortcut registration for dropdown and context menus.
 * Coordinates with the central input service to provide scoped modal navigation.
 */

import { createSignal, createEffect, onCleanup, Accessor } from 'solid-js';
import { useInput, createShortcut, SCOPE_PRIORITIES } from '../../../core/input';
import { DropdownMenuItem } from './types';

/**
 * Hook to manage keyboard navigation and focus state within a dropdown menu.
 * Integrates with the core input system to ensure modal behavior and shortcut safety.
 *
 * @param {Accessor<DropdownMenuItem[]>} items - Reactive accessor for the list of menu items.
 * @param {Function} onClose - Callback to close the menu tree.
 * @returns {Object} Object containing focus signals and state accessors.
 */
export const useMenuNavigation = (items: Accessor<DropdownMenuItem[]>, onClose: () => void) => {
    const inputService = useInput();

    /** Index of the currently focused item in the flat list of visible items. */
    const [focusedItemIndex, setFocusedItemIndex] = createSignal<number>(-1);

    /** Whether the menu is currently being navigated via keyboard (used for visual hints). */
    const [isKeyboardFocusActive, setIsKeyboardFocusActive] = createSignal(false);

    /**
     * Filters out non-interactive items (separators, labels, disabled items)
     * to determine which indices are reachable via keyboard navigation.
     *
     * @returns {number[]} Array of selectable item indices.
     */
    const getSelectableIndices = () => {
        return items()
            .map((item, itemIndex) => {
                const isInteractive = item.type !== 'separator' && item.type !== 'label';
                const isEnabled = !('disabled' in item && item.disabled);
                return isInteractive && isEnabled ? itemIndex : -1;
            })
            .filter(index => index !== -1);
    };

    /**
     * Moves focus to the next or previous selectable item.
     *
     * @param {number} direction - Navigation direction: 1 for forward/down, -1 for backward/up.
     */
    const moveFocus = (direction: 1 | -1) => {
        const selectableIndices = getSelectableIndices();
        if (selectableIndices.length === 0) return;

        const currentFocusedIndex = focusedItemIndex();
        const currentSelectablePosition = selectableIndices.indexOf(currentFocusedIndex);

        let nextPosition = currentSelectablePosition + direction;

        // Loop around if focus reaches the beginning or end of the list
        if (nextPosition >= selectableIndices.length) nextPosition = 0;
        if (nextPosition < 0) nextPosition = selectableIndices.length - 1;

        setFocusedItemIndex(selectableIndices[nextPosition]);
        setIsKeyboardFocusActive(true);
    };

    /**
     * Triggers the action associated with the currently focused item.
     * Handles standard actions and checkbox toggles.
     */
    const triggerFocusedItem = () => {
        const currentIndex = focusedItemIndex();
        if (currentIndex === -1) return;

        const activeItem = items()[currentIndex];
        if (!activeItem) return;

        if (activeItem.type === 'item') {
            activeItem.action();
            onClose();
        } else if (activeItem.type === 'checkbox') {
            activeItem.onCheckedChange(!activeItem.checked);
        }
    };

    // Shortcut registration automatically handled by the input system
    createShortcut({
        keys: 'ArrowDown',
        scope: 'menu',
        priority: 1510,
        system: true,
        action: () => moveFocus(1),
        preventDefault: true
    });

    createShortcut({
        keys: 'ArrowUp',
        scope: 'menu',
        priority: 1510,
        system: true,
        action: () => moveFocus(-1),
        preventDefault: true
    });

    createShortcut({
        keys: 'Home',
        scope: 'menu',
        priority: 1510,
        system: true,
        action: () => {
            const firstSelectableIndex = getSelectableIndices()[0];
            if (firstSelectableIndex !== undefined) setFocusedItemIndex(firstSelectableIndex);
        },
        preventDefault: true
    });

    createShortcut({
        keys: 'End',
        scope: 'menu',
        priority: 1510,
        system: true,
        action: () => {
            const selectableIndices = getSelectableIndices();
            const lastSelectableIndex = selectableIndices[selectableIndices.length - 1];
            if (lastSelectableIndex !== undefined) setFocusedItemIndex(lastSelectableIndex);
        },
        preventDefault: true
    });

    createShortcut({
        keys: ['Enter', 'Space'],
        scope: 'menu',
        priority: 1510,
        system: true,
        action: () => triggerFocusedItem(),
        preventDefault: true
    });

    createShortcut({
        keys: 'Escape',
        scope: 'menu',
        priority: 1510,
        system: true,
        action: () => onClose(),
        preventDefault: true
    });

    /**
     * Activate the 'menu' scope when the menu is open to block global shortcuts safely.
     * Use a high priority to ensure it takes precedence over lower layers.
     */
    createEffect(() => {
        inputService.pushScope('menu', SCOPE_PRIORITIES.modal + 100, true);
        onCleanup(() => inputService.popScope('menu'));
    });

    return {
        focusedItemIndex,
        setFocusedItemIndex,
        isKeyboardFocusActive,
        setIsKeyboardFocusActive
    };
};
