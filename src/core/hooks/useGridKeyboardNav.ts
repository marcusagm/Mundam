/**
 * useGridKeyboardNav
 *
 * Keyboard navigation hook for grid-based viewports (Masonry, Grid).
 * Provides arrow key navigation, selection, and scroll-to-focus functionality.
 *
 * Designed to work with virtualized layouts where items are positioned absolutely.
 *
 * Keyboard shortcuts:
 * - Arrow keys: Navigate between items
 * - Home/End: Go to first/last item
 * - Space: Toggle selection (Shift+Space to add to selection)
 * - Enter: Open item
 */

import { createSignal, createEffect, on, Accessor } from 'solid-js';
import { viewportState, viewportActions } from '../store/viewportStore';
import { useCommands, createConditionalScope } from '../input';
import type { ShortcutPayload } from '../input/types';
import type { ItemPosition } from '../viewport';
import { findByIndex, findBestCandidate, extractSelectionModifiers } from './gridNavHelpers';

function getMotionAwareScrollBehavior(): ScrollBehavior {
    if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return 'auto';
    return window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth';
}

export interface GridKeyboardNavOptions {
    /** Array of visible items with positions */
    visibleItems: Accessor<ItemPosition[]>;
    /** All items (for navigation beyond visible) */
    allItems: Accessor<{ id: string }[]>;
    /** Container height for scroll calculations */
    containerHeight: Accessor<number>;
    /** Reference to scroll container */
    scrollContainer: Accessor<HTMLDivElement | undefined>;
    /** Callback when item should be selected */
    onSelect: (id: string, modifiers: { multi: boolean; shift: boolean }) => void;
    /** Callback to select every item currently shown by the viewport. */
    onSelectAll: () => void;
    /** Callback to clear the current selection. */
    onDeselectAll: () => void;
    /** Callback to move selected or focused items to the trash. */
    onMoveToTrash: (focusedId: string | null) => void;
    /** Callback when item should be opened */
    onOpen: (id: string) => void;
    /** Check if item is selected */
    isSelected: (id: string) => boolean;
    /** Get current selection */
    getSelectedIds: () => string[];
    /** Optional callback to get exact item position (e.g. from worker) */
    getItemRect?: (id: string) => Promise<ItemPosition | null>;
}

export interface GridKeyboardNavResult {
    /** Currently focused item ID */
    focusedId: Accessor<string | null>;
    /** Set focused item */
    setFocusedId: (id: string | null) => void;
    /** Sync focus with click selection */
    syncFocusWithClick: (id: string) => void;
}

export function useGridKeyboardNav(options: GridKeyboardNavOptions): GridKeyboardNavResult {
    const [focusedId, setFocusedId] = createSignal<string | null>(null);

    // Activate viewport scope when items exist
    createConditionalScope('viewport', () => options.allItems().length > 0);

    // Find item's position in allItems array
    const getItemIndex = (id: string): number => {
        return options.allItems().findIndex(item => item.id === id);
    };

    // Sync focus when clicking an item
    const syncFocusWithClick = (id: string) => {
        setFocusedId(id);
    };

    // Find visually adjacent items based on position
    const findAdjacentItem = (
        currentId: string,
        direction: 'up' | 'down' | 'left' | 'right'
    ): string | null => {
        const visibleItems = options.visibleItems();
        const currentPos = visibleItems.find(position => position.id === currentId);

        if (!currentPos) {
            return findByIndex(currentId, direction, options.allItems());
        }

        const bestCandidate = findBestCandidate(currentPos, direction, visibleItems);

        if (!bestCandidate) {
            return findByIndex(currentId, direction, options.allItems());
        }

        return bestCandidate.id;
    };

    // Scroll to make focused item visible
    const scrollToItem = async (id: string) => {
        const container = options.scrollContainer();
        if (!container) return;
        const scrollBehavior = getMotionAwareScrollBehavior();

        // First try visible items (sync)
        let pos: ItemPosition | null | undefined = options.visibleItems().find(p => p.id === id);

        // If not visible and we have a query capability, try that (async)
        if (!pos && options.getItemRect) {
            try {
                pos = await options.getItemRect(id);
            } catch (e) {
                console.warn('Failed to get item rect', e);
            }
        }

        if (!pos) {
            // Item not visible and no rect found, estimate position (Fallback)
            const allItems = options.allItems();
            const index = getItemIndex(id);
            if (index === -1) return;

            // Estimate based on average item height
            const avgHeight = container.scrollHeight / allItems.length;
            const estimatedTop = index * avgHeight;

            container.scrollTo({
                top: estimatedTop - container.clientHeight / 2,
                behavior: scrollBehavior
            });
            return;
        }

        const containerRect = container.getBoundingClientRect();
        const scrollTop = container.scrollTop;

        // Use the found position (either from visible items or worker)
        const itemTop = pos.y;
        // For worker results, height might be available. If not, assume default.
        const itemHeight = pos.height || 280;
        const itemBottom = itemTop + itemHeight;

        // Check if item is fully visible
        if (itemTop < scrollTop) {
            // Scroll up to show item
            container.scrollTo({
                top: itemTop - 16,
                behavior: scrollBehavior
            });
        } else if (itemBottom > scrollTop + containerRect.height) {
            // Scroll down to show item
            container.scrollTo({
                top: itemBottom - containerRect.height + 16,
                behavior: scrollBehavior
            });
        }
    };

    // Auto-scroll when focused item changes
    createEffect(
        on(
            focusedId,
            id => {
                if (id !== null) {
                    scrollToItem(id);
                }
            },
            { defer: true }
        )
    );

    // Honor external scroll requests (e.g. navigation from Home) once the item is laid out
    createEffect(() => {
        const targetId = viewportState.scrollTargetId;
        if (targetId === null) return;
        if (options.visibleItems().some(position => position.id === targetId)) {
            viewportActions.requestScrollToAsset(null);
            setFocusedId(targetId);
            scrollToItem(targetId);
        }
    });

    // --- ACTIONS ---

    const move = (direction: 'up' | 'down' | 'left' | 'right') => {
        const allItems = options.allItems();
        if (allItems.length === 0) return;

        const current = focusedId();
        if (current === null) {
            setFocusedId(allItems[0].id);
            return;
        }

        const next = findAdjacentItem(current, direction);
        if (next !== null && next !== current) {
            setFocusedId(next);
        }
    };

    const actions = {
        moveUp: () => move('up'),
        moveDown: () => move('down'),
        moveLeft: () => move('left'),
        moveRight: () => move('right'),

        home: () => {
            const allItems = options.allItems();
            if (allItems.length > 0) setFocusedId(allItems[0].id);
        },

        end: () => {
            const allItems = options.allItems();
            if (allItems.length > 0) setFocusedId(allItems[allItems.length - 1].id);
        },

        open: () => {
            const current = focusedId();
            if (current !== null) options.onOpen(current);
        },

        toggleSelect: (argument?: Event | ShortcutPayload) => {
            const current = focusedId();
            if (current !== null) {
                const modifiers = extractSelectionModifiers(argument);
                options.onSelect(current, modifiers);
            }
        },
        addToSelection: () => {
            const current = focusedId();
            if (current !== null) options.onSelect(current, { multi: true, shift: false });
        },
        selectAll: options.onSelectAll,
        deselectAll: options.onDeselectAll,
        moveToTrash: () => options.onMoveToTrash(focusedId())
    };

    // Register Shortcuts
    // Subscribe to navigation commands
    useCommands({
        'viewport:move-up': actions.moveUp,
        'viewport:move-down': actions.moveDown,
        'viewport:move-left': actions.moveLeft,
        'viewport:move-right': actions.moveRight,
        'viewport:home': actions.home,
        'viewport:end': actions.end,
        'viewport:toggle-select': actions.toggleSelect,
        'viewport:open': actions.open,
        'viewport:select-add': actions.addToSelection,
        'viewport:select-all': actions.selectAll,
        'viewport:deselect-all': actions.deselectAll,
        'viewport:move-to-trash': actions.moveToTrash
    });

    return {
        focusedId,
        setFocusedId,
        syncFocusWithClick
    };
}
