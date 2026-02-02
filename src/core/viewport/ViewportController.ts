/**
 * ViewportController
 * 
 * Orchestrates communication between the main thread and the layout worker.
 * Exposes reactive signals for SolidJS components to consume.
 * 
 * Usage:
 *   const controller = createViewportController();
 *   controller.setItems([...]);
 *   controller.handleScroll(scrollTop, viewportHeight);
 *   
 *   // In component:
 *   <For each={controller.visibleItems()}>
 */

import { createSignal, batch } from "solid-js";
import type {
  LayoutItemInput,
  LayoutConfig,
  ItemPosition,
  IViewportController,
  WorkerOutMessage,
  LayoutMode,
} from "./types";

// Import worker with Vite's native worker support
import LayoutWorker from "./layout.worker?worker";

// ============================================================================
// Default Configuration
// ============================================================================

const DEFAULT_CONFIG: LayoutConfig = {
  mode: "masonry",
  containerWidth: 0,
  itemSize: 280,
  gap: 16,
  buffer: 1000,
};

// ============================================================================
// Controller Implementation
// ============================================================================

export class ViewportController implements IViewportController {
  private worker: Worker;
  private config: LayoutConfig;
  private disposed = false;

  // Reactive signals
  private readonly _visibleItems = createSignal<ItemPosition[]>([]);
  private readonly _totalHeight = createSignal(0);
  private readonly _isCalculating = createSignal(false);

  // Throttle scroll updates for performance
  private scrollRAF: number | null = null;
  private pendingScroll: { scrollTop: number; viewportHeight: number } | null = null;
  // Store last known scroll position to re-query visibility after layout changes
  private lastScroll: { scrollTop: number; viewportHeight: number } = { scrollTop: 0, viewportHeight: 800 };
  // Debounce resize for smoother performance
  private resizeTimeout: ReturnType<typeof setTimeout> | null = null;

  constructor(initialConfig: Partial<LayoutConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...initialConfig };
    this.worker = new LayoutWorker();
    this.setupWorkerListeners();

    // Send initial config
    this.postMessage({ type: "CONFIGURE", payload: this.config });
  }

  // ============================================================================
  // Public API - Read-only Signals
  // ============================================================================

  get visibleItems(): () => ItemPosition[] {
    return this._visibleItems[0];
  }

  get totalHeight(): () => number {
    return this._totalHeight[0];
  }

  get isCalculating(): () => boolean {
    return this._isCalculating[0];
  }

  // ============================================================================
  // Public API - Commands
  // ============================================================================

  /**
   * Sets the items to be laid out.
   * Triggers a full layout recalculation in the worker.
   */
  setItems(items: LayoutItemInput[]): void {
    if (this.disposed) return;
    
    this._isCalculating[1](true);
    this.postMessage({ type: "SET_ITEMS", payload: items });
  }

  /**
   * Updates layout configuration.
   * Only changed properties need to be passed.
   */
  setConfig(config: Partial<LayoutConfig>): void {
    if (this.disposed) return;

    // Merge with current config
    this.config = { ...this.config, ...config };
    
    this._isCalculating[1](true);
    this.postMessage({ type: "CONFIGURE", payload: this.config });
  }

  /**
   * Handles container resize.
   * Debounced to prevent excessive recalculations during continuous resize.
   */
  handleResize(width: number): void {
    if (this.disposed) return;
    if (Math.abs(this.config.containerWidth - width) <= 1) return;

    this.config.containerWidth = width;
    
    // Debounce resize to prevent excessive worker messages
    if (this.resizeTimeout) {
      clearTimeout(this.resizeTimeout);
    }
    
    this.resizeTimeout = setTimeout(() => {
      this.resizeTimeout = null;
      this._isCalculating[1](true);
      this.postMessage({ type: "RESIZE", payload: { width: this.config.containerWidth } });
    }, 50); // 50ms debounce
  }

  /**
   * Handles scroll position changes.
   * Throttled via requestAnimationFrame for smooth performance.
   */
  handleScroll(scrollTop: number, viewportHeight: number): void {
    if (this.disposed) return;

    // Store as last known scroll
    this.lastScroll = { scrollTop, viewportHeight };
    // Store pending scroll data
    this.pendingScroll = { scrollTop, viewportHeight };

    // Throttle to animation frame
    if (this.scrollRAF !== null) return;

    this.scrollRAF = requestAnimationFrame(() => {
      this.scrollRAF = null;
      
      if (this.pendingScroll) {
        this.postMessage({ type: "SCROLL", payload: this.pendingScroll });
        this.pendingScroll = null;
      }
    });
  }

  /**
   * Cleans up worker and cancels pending operations.
   * Call this when the component unmounts.
   */
  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;

    if (this.scrollRAF !== null) {
      cancelAnimationFrame(this.scrollRAF);
    }
    
    if (this.resizeTimeout !== null) {
      clearTimeout(this.resizeTimeout);
    }

    this.worker.terminate();
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private setupWorkerListeners(): void {
    this.worker.onmessage = (e: MessageEvent<WorkerOutMessage>) => {
      if (this.disposed) return;

      const { type, payload } = e.data;

      switch (type) {
        case "LAYOUT_COMPLETE":
          batch(() => {
            this._totalHeight[1](payload.totalHeight);
            this._isCalculating[1](false);
          });
          
          // Dev mode: log layout stats
          if (import.meta.env.DEV) {
            console.debug(
              `[Viewport] Layout complete: ${payload.totalHeight}px total height`
            );
          }
          
          // CRITICAL: Always re-query visibility after layout changes
          // This fixes the issue where resize/toggle panels don't update until scroll
          this.postMessage({ type: "SCROLL", payload: this.lastScroll });
          break;

        case "VISIBLE_UPDATE":
          // Always update - positions may change even with same IDs during resize
          this._visibleItems[1](payload as ItemPosition[]);
          break;

        case "ERROR":
          console.error("[ViewportController] Worker error:", payload.message);
          this._isCalculating[1](false);
          break;
      }
    };

    this.worker.onerror = (error) => {
      console.error("[ViewportController] Worker crashed:", error);
      this._isCalculating[1](false);
    };
  }

  private postMessage(message: { type: string; payload?: unknown }): void {
    if (!this.disposed) {
      this.worker.postMessage(message);
    }
  }
}

// ============================================================================
// Factory Function
// ============================================================================

/**
 * Creates a new ViewportController instance.
 * Prefer using this over `new ViewportController()` for consistency.
 */
export function createViewportController(
  mode: LayoutMode = "masonry",
  initialConfig: Partial<LayoutConfig> = {}
): ViewportController {
  return new ViewportController({ mode, ...initialConfig });
}
