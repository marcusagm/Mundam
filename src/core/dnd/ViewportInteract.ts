/**
 * ViewportInteract
 * 
 * Coordinate-based hit testing for virtualized viewports.
 * Instead of relying on DOM events on each item, we calculate which item
 * is under the cursor based on the layout positions from the Worker.
 * 
 * This enables DnD to work correctly even when items are virtualized.
 */

import type { ItemPosition } from "../viewport";

export interface HitTestResult {
  /** ID of the item under the cursor, or null if none */
  targetId: number | null;
  /** Position relative to the target: "before", "inside", "after" */
  position: "before" | "inside" | "after";
  /** The matched item position for visual feedback */
  targetPosition: ItemPosition | null;
}

/**
 * Service for coordinate-based hit testing in virtualized viewports.
 */
export class ViewportInteract {
  private items: ItemPosition[] = [];
  private containerRect: DOMRect | null = null;
  private scrollTop: number = 0;

  /**
   * Update the list of visible items from the Worker.
   */
  setVisibleItems(items: ItemPosition[]): void {
    this.items = items;
  }

  /**
   * Update container dimensions.
   */
  setContainerRect(rect: DOMRect): void {
    this.containerRect = rect;
  }

  /**
   * Update scroll position.
   */
  setScrollTop(scrollTop: number): void {
    this.scrollTop = scrollTop;
  }

  /**
   * Perform hit testing to find which item is under the given client coordinates.
   * 
   * @param clientX - Mouse X relative to viewport
   * @param clientY - Mouse Y relative to viewport
   * @returns Hit test result with target ID and position
   */
  hitTest(clientX: number, clientY: number): HitTestResult {
    if (!this.containerRect || this.items.length === 0) {
      return { targetId: null, position: "inside", targetPosition: null };
    }

    // Convert client coordinates to container-relative coordinates
    const relativeX = clientX - this.containerRect.left;
    const relativeY = clientY - this.containerRect.top + this.scrollTop;

    // Find item under cursor
    for (const item of this.items) {
      const isInXRange = relativeX >= item.x && relativeX <= item.x + item.width;
      const isInYRange = relativeY >= item.y && relativeY <= item.y + item.height;

      if (isInXRange && isInYRange) {
        // Determine position within item (for reordering feedback)
        const itemCenterY = item.y + item.height / 2;
        const position = relativeY < itemCenterY ? "before" : "after";

        return {
          targetId: item.id,
          position,
          targetPosition: item,
        };
      }
    }

    return { targetId: null, position: "inside", targetPosition: null };
  }

  /**
   * Find the closest item to the given coordinates (for edge cases).
   */
  findClosest(clientX: number, clientY: number): HitTestResult {
    if (!this.containerRect || this.items.length === 0) {
      return { targetId: null, position: "inside", targetPosition: null };
    }

    const relativeX = clientX - this.containerRect.left;
    const relativeY = clientY - this.containerRect.top + this.scrollTop;

    let closestItem: ItemPosition | null = null;
    let closestDistance = Infinity;

    for (const item of this.items) {
      const centerX = item.x + item.width / 2;
      const centerY = item.y + item.height / 2;
      const distance = Math.sqrt(
        Math.pow(relativeX - centerX, 2) + Math.pow(relativeY - centerY, 2)
      );

      if (distance < closestDistance) {
        closestDistance = distance;
        closestItem = item;
      }
    }

    if (closestItem) {
      const itemCenterY = closestItem.y + closestItem.height / 2;
      const position = relativeY < itemCenterY ? "before" : "after";

      return {
        targetId: closestItem.id,
        position,
        targetPosition: closestItem,
      };
    }

    return { targetId: null, position: "inside", targetPosition: null };
  }

  /**
   * Get the position for a drop indicator line.
   * 
   * @param targetPosition - The target item position
   * @param position - "before" or "after"
   * @returns CSS styles for the indicator
   */
  getDropIndicatorStyle(
    targetPosition: ItemPosition,
    position: "before" | "after"
  ): { top: number; left: number; width: number } {
    const y = position === "before" 
      ? targetPosition.y - 2 
      : targetPosition.y + targetPosition.height + 2;

    return {
      top: y,
      left: targetPosition.x,
      width: targetPosition.width,
    };
  }
}

/**
 * Factory function to create a ViewportInteract instance.
 */
export function createViewportInteract(): ViewportInteract {
  return new ViewportInteract();
}
