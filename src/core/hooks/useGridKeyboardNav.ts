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

import { createSignal, createEffect, on, Accessor } from "solid-js";
import { useShortcuts, createConditionalScope } from "../input";
import type { ItemPosition } from "../viewport";

export interface GridKeyboardNavOptions {
  /** Array of visible items with positions */
  visibleItems: Accessor<ItemPosition[]>;
  /** All items (for navigation beyond visible) */
  allItems: Accessor<{ id: number }[]>;
  /** Container height for scroll calculations */
  containerHeight: Accessor<number>;
  /** Reference to scroll container */
  scrollContainer: Accessor<HTMLDivElement | undefined>;
  /** Callback when item should be selected */
  onSelect: (id: number, multi: boolean) => void;
  /** Callback when item should be opened */
  onOpen: (id: number) => void;
  /** Check if item is selected */
  isSelected: (id: number) => boolean;
  /** Get current selection */
  getSelectedIds: () => (number | string)[];
  /** Optional callback to get exact item position (e.g. from worker) */
  getItemRect?: (id: number) => Promise<ItemPosition | null>;
}

export interface GridKeyboardNavResult {
  /** Currently focused item ID */
  focusedId: Accessor<number | null>;
  /** Set focused item */
  setFocusedId: (id: number | null) => void;
  /** Sync focus with click selection */
  syncFocusWithClick: (id: number) => void;
}

export function useGridKeyboardNav(
  options: GridKeyboardNavOptions
): GridKeyboardNavResult {
  const [focusedId, setFocusedId] = createSignal<number | null>(null);

  // Activate viewport scope when items exist
  createConditionalScope('viewport', () => options.allItems().length > 0);

  // Find item's position in allItems array
  const getItemIndex = (id: number): number => {
    return options.allItems().findIndex(item => item.id === id);
  };

  // Sync focus when clicking an item
  const syncFocusWithClick = (id: number) => {
    setFocusedId(id);
  };

  // Find visually adjacent items based on position
  const findAdjacentItem = (
    currentId: number,
    direction: "up" | "down" | "left" | "right"
  ): number | null => {
    const visibleItems = options.visibleItems();
    const currentPos = visibleItems.find(p => p.id === currentId);
    
    if (!currentPos) {
      // Current item not visible, try to find by index
      const allItems = options.allItems();
      const currentIndex = getItemIndex(currentId);
      
      if (direction === "up" || direction === "left") {
        return currentIndex > 0 ? allItems[currentIndex - 1].id : null;
      } else {
        return currentIndex < allItems.length - 1 ? allItems[currentIndex + 1].id : null;
      }
    }

    // Find the best candidate based on position
    let bestCandidate: ItemPosition | null = null;
    let bestScore = Infinity;

    const centerX = currentPos.x + currentPos.width / 2;
    const centerY = currentPos.y + currentPos.height / 2;

    for (const pos of visibleItems) {
      if (pos.id === currentId) continue;

      const posCenterX = pos.x + pos.width / 2;
      const posCenterY = pos.y + pos.height / 2;

      let isValidDirection = false;
      let score = 0;

      switch (direction) {
        case "up":
          if (posCenterY < centerY - 10) {
            isValidDirection = true;
            // Primary score: strictly geometric distance
            // Weight Y distance more heavily for column retention
            score = Math.abs(posCenterX - centerX) * 2 + Math.abs(centerY - posCenterY);
          }
          break;
        case "down":
          if (posCenterY > centerY + 10) {
            isValidDirection = true;
            score = Math.abs(posCenterX - centerX) * 2 + Math.abs(posCenterY - centerY);
          }
          break;
        case "left":
          if (posCenterX < centerX - 10) {
            isValidDirection = true;
            score = Math.abs(posCenterY - centerY) * 2 + Math.abs(centerX - posCenterX);
          }
          break;
        case "right":
          if (posCenterX > centerX + 10) {
            isValidDirection = true;
            score = Math.abs(posCenterY - centerY) * 2 + Math.abs(posCenterX - centerX);
          }
          break;
      }

      if (isValidDirection) {
          if (score < bestScore) {
              bestScore = score;
              bestCandidate = pos;
          }
      }
    }

    // If no candidate found in visible items, try adjacent index
    if (!bestCandidate) {
      const allItems = options.allItems();
      const currentIndex = getItemIndex(currentId);
      
      if (direction === "up" || direction === "left") {
        return currentIndex > 0 ? allItems[currentIndex - 1].id : null;
      } else {
        return currentIndex < allItems.length - 1 ? allItems[currentIndex + 1].id : null;
      }
    }

    return bestCandidate.id;
  };

  // Scroll to make focused item visible
  const scrollToItem = async (id: number) => {
    const container = options.scrollContainer();
    if (!container) return;

    // First try visible items (sync)
    let pos: ItemPosition | null | undefined = options.visibleItems().find(p => p.id === id);
    
    // If not visible and we have a query capability, try that (async)
    if (!pos && options.getItemRect) {
      try {
        pos = await options.getItemRect(id);
      } catch (e) {
        console.warn("Failed to get item rect", e);
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
        behavior: "smooth"
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
        behavior: "smooth"
      });
    } else if (itemBottom > scrollTop + containerRect.height) {
      // Scroll down to show item
      container.scrollTo({
        top: itemBottom - containerRect.height + 16,
        behavior: "smooth"
      });
    }
  };

  // Auto-scroll when focused item changes
  createEffect(
    on(focusedId, (id) => {
      if (id !== null) {
        scrollToItem(id);
      }
    }, { defer: true })
  );

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
      
      toggleSelect: (e?: Event | null) => {
          const current = focusedId();
          if (current !== null) {
              const multi = e && (e as KeyboardEvent).shiftKey ? (e as KeyboardEvent).shiftKey : false;
              options.onSelect(current, multi);
          }
      }
  };

  // Register Shortcuts
  useShortcuts([
      { keys: 'ArrowUp', name: 'Move Up', scope: 'viewport', action: actions.moveUp },
      { keys: 'ArrowDown', name: 'Move Down', scope: 'viewport', action: actions.moveDown },
      { keys: 'ArrowLeft', name: 'Move Left', scope: 'viewport', action: actions.moveLeft },
      { keys: 'ArrowRight', name: 'Move Right', scope: 'viewport', action: actions.moveRight },
      { keys: 'Home', name: 'Go to Start', scope: 'viewport', action: actions.home },
      { keys: 'End', name: 'Go to End', scope: 'viewport', action: actions.end },
      { keys: 'Space', name: 'Toggle Selection', scope: 'viewport', action: actions.toggleSelect },
      { keys: 'Enter', name: 'Open Item', scope: 'viewport', action: actions.open },
  ]);

  return {
    focusedId,
    setFocusedId,
    syncFocusWithClick,
  };
}
