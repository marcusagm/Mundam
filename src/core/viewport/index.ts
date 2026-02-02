/**
 * Viewport System Public API
 * 
 * This module provides a high-performance virtualization system for
 * rendering large lists of items (images) in both masonry and grid layouts.
 * 
 * Architecture:
 * - Web Worker handles all layout calculations off the main thread
 * - Spatial Grid enables O(1) visibility queries during scroll
 * - ViewportController provides reactive SolidJS signals
 * 
 * Usage:
 *   import { createViewportController, type ItemPosition } from "@/core/viewport";
 *   
 *   const controller = createViewportController("masonry");
 *   controller.setItems(items.map(i => ({ id: i.id, aspectRatio: i.width / i.height })));
 *   
 *   // In component:
 *   <For each={controller.visibleItems()}>
 *     {(pos) => <Item style={{ transform: `translate(${pos.x}px, ${pos.y}px)` }} />}
 *   </For>
 */

// Types
export type {
  LayoutItemInput,
  LayoutMode,
  LayoutConfig,
  ItemPosition,
  VirtualizationResult,
  IViewportController,
} from "./types";

// Controller
export { ViewportController, createViewportController } from "./ViewportController";
