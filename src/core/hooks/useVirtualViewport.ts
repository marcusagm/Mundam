/**
 * useVirtualViewport
 * 
 * Shared hook for virtualized viewport management.
 * Creates and manages a ViewportController instance connected to the layout Worker.
 * 
 * Usage:
 *   const { visibleItems, totalHeight, ... } = useVirtualViewport("masonry", items);
 */

import { createEffect, onCleanup, on } from "solid-js";
import { createViewportController, type LayoutMode, type LayoutItemInput, type ItemPosition } from "../viewport";
import { useFilters } from "./useFilters";

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
}

/**
 * Creates a virtualized viewport connected to the layout Worker.
 * 
 * @param mode - "masonry" or "grid" layout mode
 * @param items - Reactive accessor for the items array
 * @param options - Optional configuration
 */
export function useVirtualViewport(
  mode: LayoutMode,
  items: () => LayoutItemInput[],
  options: UseVirtualViewportOptions = {}
): VirtualViewportResult {
  const filters = useFilters();
  // Increased buffer to reduce flickering on minimal scroll
  const { gap = 16, buffer = 1500 } = options;

  // Create controller instance
  const controller = createViewportController(mode, {
    gap,
    buffer,
    itemSize: filters.thumbSize || 280,
  });

  // Sync config when thumbSize changes
  createEffect(
    on(
      () => filters.thumbSize,
      (size) => {
        if (size && size > 0) {
          controller.setConfig({ itemSize: size });
        }
      },
      { defer: true }
    )
  );

  // Sync items when they change
  createEffect(
    on(items, (currentItems) => {
      controller.setItems(currentItems);
    })
  );

  // Cleanup on unmount
  onCleanup(() => {
    controller.dispose();
  });

  return {
    visibleItems: controller.visibleItems,
    totalHeight: controller.totalHeight,
    isCalculating: controller.isCalculating,
    handleResize: (width) => controller.handleResize(width),
    handleScroll: (scrollTop, viewportHeight) => controller.handleScroll(scrollTop, viewportHeight),
    invalidate: () => controller.setConfig({}), // Empty config triggers recalculation
  };
}

/**
 * Helper to convert ImageItem to LayoutItemInput for the Worker.
 * Only sends minimal data needed for layout calculation.
 */
export function toLayoutItems<T extends { id: number; width?: number | null; height?: number | null }>(
  items: T[]
): LayoutItemInput[] {
  return items.map((item) => ({
    id: item.id,
    aspectRatio:
      item.width && item.height && item.height > 0
        ? item.width / item.height
        : 1, // Default to square if no dimensions
  }));
}
