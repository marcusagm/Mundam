/**
 * Viewport System Types
 * 
 * Shared type definitions for the Web Worker-based virtualization engine.
 * These types define the contract between the main thread and the worker.
 */

// ============================================================================
// Layout Input Types
// ============================================================================

/**
 * Minimal item data needed by the worker for layout calculation.
 * Keep this lean for fast serialization over postMessage.
 */
export interface LayoutItemInput {
  id: number;
  aspectRatio: number; // width / height (1 for grid mode)
}

/**
 * Layout mode determines the algorithm used.
 * - masonry-v: Vertical masonry - fixed column width, variable height (Pinterest-style)
 * - masonry-h: Horizontal masonry - fixed row height, variable width (Flickr-style)
 * - grid: Uniform square items, simple row/column calculation
 * - masonry: Alias for masonry-v (backwards compatibility)
 */
export type LayoutMode = "masonry" | "masonry-v" | "masonry-h" | "grid";

/**
 * Configuration for layout calculation.
 */
export interface LayoutConfig {
  mode: LayoutMode;
  containerWidth: number;
  itemSize: number; // minColumnWidth for masonry, baseSize for grid
  gap: number;
  buffer: number; // Extra pixels to render above/below viewport
}

// ============================================================================
// Layout Output Types
// ============================================================================

/**
 * Calculated position and dimensions for a single item.
 */
export interface ItemPosition {
  id: number;
  x: number;
  y: number;
  width: number;
  height: number;
}

/**
 * Complete virtualization result sent back from worker.
 */
export interface VirtualizationResult {
  totalHeight: number;
  visibleItems: ItemPosition[];
}

// ============================================================================
// Worker Message Protocol
// ============================================================================

/**
 * Messages sent FROM main thread TO worker.
 */
export type WorkerInMessage =
  | { type: "SET_ITEMS"; payload: LayoutItemInput[] }
  | { type: "CONFIGURE"; payload: LayoutConfig }
  | { type: "RESIZE"; payload: { width: number } }
  | { type: "SCROLL"; payload: { scrollTop: number; viewportHeight: number } }
  | { type: "INVALIDATE" } // Force recalculation
  | { type: "QUERY_POSITION"; payload: { id: number; requestId: string } };

/**
 * Messages sent FROM worker TO main thread.
 */
export type WorkerOutMessage =
  | { type: "LAYOUT_COMPLETE"; payload: { totalHeight: number } }
  | { type: "VISIBLE_UPDATE"; payload: ItemPosition[] }
  | { type: "ERROR"; payload: { message: string } }
  | { type: "POSITION_RESULT"; payload: { requestId: string; position: ItemPosition | null } };

// ============================================================================
// Controller Types
// ============================================================================

/**
 * Public API exposed by ViewportController.
 */
export interface IViewportController {
  // Reactive signals (read-only from outside)
  readonly visibleItems: () => ItemPosition[];
  readonly totalHeight: () => number;
  readonly isCalculating: () => boolean;

  // Commands
  setItems(items: LayoutItemInput[]): void;
  setConfig(config: Partial<LayoutConfig>): void;
  handleResize(width: number): void;
  handleScroll(scrollTop: number, viewportHeight: number): void;
  getItemPosition(id: number): Promise<ItemPosition | null>;
  dispose(): void;
}

// ============================================================================
// Spatial Grid Types (Internal to Worker)
// ============================================================================

/**
 * Spatial grid cell containing item IDs.
 * Used for O(1) visibility queries.
 */
export interface SpatialCell {
  startY: number;
  endY: number;
  itemIds: number[];
}

/**
 * Configuration for the spatial grid.
 */
export interface SpatialGridConfig {
  cellHeight: number; // Height of each cell (default: 1000px)
}
