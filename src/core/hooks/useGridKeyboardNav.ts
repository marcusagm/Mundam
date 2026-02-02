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
import { shortcutStore } from "../input/store/shortcutStore";
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
  /** Handle keydown events */
  handleKeyDown: (e: KeyboardEvent) => void;
  /** Sync focus with click selection */
  syncFocusWithClick: (id: number) => void;
}

export function useGridKeyboardNav(
  options: GridKeyboardNavOptions
): GridKeyboardNavResult {
  const [focusedId, setFocusedId] = createSignal<number | null>(null);

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
            // Prefer items directly above (smaller X distance)
            score = Math.abs(posCenterX - centerX) + (centerY - posCenterY) * 0.1;
          }
          break;
        case "down":
          if (posCenterY > centerY + 10) {
            isValidDirection = true;
            score = Math.abs(posCenterX - centerX) + (posCenterY - centerY) * 0.1;
          }
          break;
        case "left":
          if (posCenterX < centerX - 10) {
            isValidDirection = true;
            // Prefer items on same row (smaller Y distance)
            score = Math.abs(posCenterY - centerY) + (centerX - posCenterX) * 0.1;
          }
          break;
        case "right":
          if (posCenterX > centerX + 10) {
            isValidDirection = true;
            score = Math.abs(posCenterY - centerY) + (posCenterX - centerX) * 0.1;
          }
          break;
      }

      if (isValidDirection && score < bestScore) {
        bestScore = score;
        bestCandidate = pos;
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

  // match helper
  const isCommand = (e: KeyboardEvent, command: string, defaultKey: string): boolean => {
    const shortcut = shortcutStore.getByCommand(command);
    // Fallback to default check if no shortcut configured (though registration ensures default)
    if (!shortcut) return e.key === defaultKey;
    
    // If keys is an array, check if any match
    const keysArray = Array.isArray(shortcut.keys) ? shortcut.keys : [shortcut.keys];
    
    // Basic check for exact key match (handling case insensitivity for letters)
    // This allows simple remapping like ArrowUp -> w or k
    return keysArray.some(k => 
      k === e.key || 
      k === e.code || // Check e.code (Important for Space: key=" ", code="Space")
      k.toLowerCase() === e.key.toLowerCase() ||
      k.replace('Key', '') === e.key.toUpperCase()
    );
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    // Ignore input fields unless explicitly allowed
    if (["INPUT", "TEXTAREA"].includes((e.target as HTMLElement).tagName)) {
        return;
    }

    const allItems = options.allItems();
    if (allItems.length === 0) return;

    const currentFocused = focusedId();
    let nextId: number | null = null;
    let handled = false;

    if (isCommand(e, 'viewport:move-up', 'ArrowUp')) {
      handled = true;
      e.preventDefault();
      if (currentFocused === null) nextId = allItems[0].id;
      else nextId = findAdjacentItem(currentFocused, "up") ?? currentFocused;
    }
    else if (isCommand(e, 'viewport:move-down', 'ArrowDown')) {
      handled = true;
      e.preventDefault();
      if (currentFocused === null) nextId = allItems[0].id;
      else nextId = findAdjacentItem(currentFocused, "down") ?? currentFocused;
    }
    else if (isCommand(e, 'viewport:move-left', 'ArrowLeft')) {
      handled = true;
      e.preventDefault();
      if (currentFocused === null) nextId = allItems[0].id;
      else nextId = findAdjacentItem(currentFocused, "left") ?? currentFocused;
    }
    else if (isCommand(e, 'viewport:move-right', 'ArrowRight')) {
      handled = true;
      e.preventDefault();
      if (currentFocused === null) nextId = allItems[0].id;
      else nextId = findAdjacentItem(currentFocused, "right") ?? currentFocused;
    }
    else if (isCommand(e, 'viewport:home', 'Home')) {
      handled = true;
      e.preventDefault();
      nextId = allItems[0].id;
    }
    else if (isCommand(e, 'viewport:end', 'End')) {
      handled = true;
      e.preventDefault();
      nextId = allItems[allItems.length - 1].id;
    }
    else if (isCommand(e, 'viewport:open', 'Enter')) {
      handled = true;
      e.preventDefault();
      if (currentFocused !== null) options.onOpen(currentFocused);
    }
    else if (isCommand(e, 'viewport:toggle-select', ' ')) {
      handled = true;
      e.preventDefault();
      if (currentFocused !== null) options.onSelect(currentFocused, e.shiftKey);
    }

    if (handled && nextId !== null && nextId !== currentFocused) {
      setFocusedId(nextId);
    }
  };

  return {
    focusedId,
    setFocusedId,
    handleKeyDown,
    syncFocusWithClick,
  };
}
