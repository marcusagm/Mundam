/**
 * useVirtualViewport
 *
 * Shared hook for virtualized viewport management.
 * Creates and manages a ViewportController instance connected to the layout Worker.
 *
 * Usage:
 *   const { visibleItems, totalHeight, ... } = useVirtualViewport("masonry", items);
 */

import { createEffect, createMemo, onCleanup, on, untrack } from 'solid-js';
import {
    createViewportController,
    type LayoutMode,
    type LayoutItemInput,
    type ItemPosition
} from '../viewport';
import { viewportState } from '../store/viewportStore';
import { useFilters } from './useFilters';
import { useLibrary } from './useLibrary';

export interface UseVirtualViewportOptions {
    gap?: number;
    buffer?: number;
}

export interface VirtualViewportResult {
    /** Currently visible item positions from the Worker */
    visibleItems: () => ItemPosition[];
    /** Total height of the virtual container */
    totalHeight: () => number;
    /** Whether the Worker is currently calculating layout */
    isCalculating: () => boolean;
    /** Notify the controller of a resize */
    handleResize: (width: number) => void;
    /** Notify the controller of a scroll */
    handleScroll: (scrollTop: number, viewportHeight: number) => void;
    /** Force a layout recalculation */
    invalidate: () => void;
    /** Query item position */
    getItemPosition: (id: string) => Promise<ItemPosition | null>;
}

/**
 * Creates a virtualized viewport connected to the layout Worker.
 *
 * @param mode - Layout mode (can be a string or accessor for reactive updates)
 * @param items - Reactive accessor for the items array
 * @param options - Optional configuration
 */
export function useVirtualViewport(
    mode: LayoutMode | (() => LayoutMode),
    items: () => LayoutItemInput[],
    options: UseVirtualViewportOptions = {}
): VirtualViewportResult {
    const filters = useFilters();
    // Increased buffer to reduce flickering on minimal scroll
    const { gap = 16, buffer = 1500 } = options;

    // Normalize mode to always be an accessor
    const getMode = typeof mode === 'function' ? mode : () => mode;

    // Create controller instance with initial mode
    const controller = createViewportController(getMode(), {
        gap,
        buffer,
        itemSize: filters.thumbSize || 280
    });

    // Sync mode when it changes (for masonry-v ↔ masonry-h switching)
    createEffect(
        on(
            getMode,
            newMode => {
                controller.setConfig({ mode: newMode });
            },
            { defer: true }
        )
    );

    // Sync config when thumbSize changes
    createEffect(
        on(
            () => filters.thumbSize,
            size => {
                if (size && size > 0) {
                    controller.setConfig({ itemSize: size });
                }
            },
            { defer: true }
        )
    );

    // Sync items when they change
    createEffect(
        on(items, currentItems => {
            controller.setItems(currentItems);
        })
    );

    // Cleanup on unmount
    onCleanup(() => {
        controller.dispose();
    });

    /* Priority Thumbnail Generation */
    const lib = useLibrary();
    // Only the set of visible IDs matters: thumbnail updates must not re-trigger the request.
    const visibleIdsKey = createMemo(() =>
        controller
            .visibleItems()
            .map(v => v.id)
            .join(',')
    );

    createEffect(
        on(visibleIdsKey, key => {
            if (!key) return;

            const debounceTimer = setTimeout(() => {
                const missingThumbnailIds = new Set(
                    untrack(() => lib.items)
                        .filter(item => !item.thumbnail_path)
                        .map(item => String(item.id))
                );
                // visibleItems includes a large buffer around the viewport; rank items so the
                // ones actually on screen come first, then the closest ones from the buffer.
                const { scrollTop, viewportHeight } = untrack(() => viewportState.scrollPosition);
                const viewportBottom = scrollTop + viewportHeight;
                const distanceToViewport = (position: ItemPosition) => {
                    if (position.y + position.height < scrollTop) {
                        return scrollTop - (position.y + position.height);
                    }
                    return position.y > viewportBottom ? position.y - viewportBottom : 0;
                };

                const idsToPrioritize = untrack(() => controller.visibleItems())
                    .filter(v => missingThumbnailIds.has(v.id))
                    .map(v => ({ id: v.id, distance: distanceToViewport(v), y: v.y, x: v.x }))
                    .sort((a, b) => a.distance - b.distance || a.y - b.y || a.x - b.x)
                    .map(v => v.id);

                if (idsToPrioritize.length > 0) {
                    lib.setThumbnailPriority(idsToPrioritize);
                }
            }, 150);

            onCleanup(() => clearTimeout(debounceTimer));
        })
    );

    return {
        visibleItems: controller.visibleItems,
        totalHeight: controller.totalHeight,
        isCalculating: controller.isCalculating,
        handleResize: width => controller.handleResize(width),
        handleScroll: (scrollTop, viewportHeight) =>
            controller.handleScroll(scrollTop, viewportHeight),
        invalidate: () => controller.setConfig({}), // Empty config triggers recalculation
        getItemPosition: id => controller.getItemPosition(id)
    };
}

/**
 * Helper to convert AssetItem to LayoutItemInput for the Worker.
 * Only sends minimal data needed for layout calculation.
 */
export function toLayoutItems<
    T extends { id: string | number; width?: number | null; height?: number | null }
>(items: T[]): LayoutItemInput[] {
    return items.map(item => ({
        id: String(item.id),
        aspectRatio: item.width && item.height && item.height > 0 ? item.width / item.height : 1 // Default to square if no dimensions
    }));
}
