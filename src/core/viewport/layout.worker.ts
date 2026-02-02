/// <reference lib="webworker" />

/**
 * Layout Worker
 * 
 * Offloads all layout calculations to a separate thread.
 * Uses a Spatial Grid for O(1) visibility queries.
 * 
 * Supports three layout modes:
 * - Masonry-V: Vertical masonry - fixed column width, variable height (Pinterest-style)
 * - Masonry-H: Horizontal masonry - fixed row height, variable width (Flickr/Google Photos-style)
 * - Grid: Uniform square cells, simple row/column calculation
 */

import type {
  LayoutItemInput,
  LayoutConfig,
  ItemPosition,
  WorkerInMessage,
  WorkerOutMessage,
} from "./types";

// ============================================================================
// Worker State
// ============================================================================

let items: LayoutItemInput[] = [];
let config: LayoutConfig = {
  mode: "masonry",
  containerWidth: 0,
  itemSize: 280,
  gap: 16,
  buffer: 1000,
};

// Layout cache: id -> position
const positions = new Map<number, ItemPosition>();

// Spatial Grid: cell index -> item IDs in that cell
const spatialGrid = new Map<number, number[]>();
const CELL_HEIGHT = 1000; // Each cell covers 1000px of vertical space

let totalHeight = 0;

// ============================================================================
// Message Handler
// ============================================================================

self.onmessage = (e: MessageEvent<WorkerInMessage>) => {
  const msg = e.data;

  try {
    switch (msg.type) {
      case "SET_ITEMS":
        items = msg.payload;
        recalculateLayout();
        break;

      case "CONFIGURE":
        config = { ...config, ...msg.payload };
        recalculateLayout();
        break;

      case "RESIZE":
        if (Math.abs(config.containerWidth - msg.payload.width) > 1) {
          config.containerWidth = msg.payload.width;
          recalculateLayout();
        }
        break;

      case "SCROLL":
        handleScroll(msg.payload.scrollTop, msg.payload.viewportHeight);
        break;

      case "INVALIDATE":
        recalculateLayout();
        break;
    }
  } catch (error) {
    const errMsg = error instanceof Error ? error.message : "Unknown error";
    respond({ type: "ERROR", payload: { message: errMsg } });
  }
};

// ============================================================================
// Layout Algorithms
// ============================================================================

function recalculateLayout(): void {
  positions.clear();
  spatialGrid.clear();

  if (config.containerWidth <= 0 || items.length === 0) {
    totalHeight = 0;
    respond({ type: "LAYOUT_COMPLETE", payload: { totalHeight: 0 } });
    return;
  }

  switch (config.mode) {
    case "masonry":
    case "masonry-v":
      calculateMasonryVerticalLayout();
      break;
    case "masonry-h":
      calculateMasonryHorizontalLayout();
      break;
    case "grid":
      calculateGridLayout();
      break;
  }

  respond({ type: "LAYOUT_COMPLETE", payload: { totalHeight } });
}

/**
 * Masonry Vertical Layout Algorithm (Pinterest-style)
 * 
 * Fixed column widths, variable heights.
 * Places each item in the shortest column, creating a Pinterest-style layout.
 * Items have variable heights based on their aspect ratio.
 */
function calculateMasonryVerticalLayout(): void {
  const { containerWidth, itemSize, gap } = config;

  // Calculate number of columns that fit
  const columnCount = Math.max(1, Math.floor((containerWidth + gap) / (itemSize + gap)));
  
  // Calculate actual column width to fill container
  const columnWidth = (containerWidth - (columnCount - 1) * gap) / columnCount;

  // Track height of each column
  const columnHeights = new Float32Array(columnCount);

  for (const item of items) {
    // Find shortest column
    let minHeight = columnHeights[0];
    let minColumn = 0;

    for (let col = 1; col < columnCount; col++) {
      if (columnHeights[col] < minHeight) {
        minHeight = columnHeights[col];
        minColumn = col;
      }
    }

    // Calculate position
    const x = minColumn * (columnWidth + gap);
    const y = minHeight;
    
    // Height based on aspect ratio (handle edge cases)
    const aspectRatio = item.aspectRatio > 0 ? item.aspectRatio : 1;
    const height = columnWidth / aspectRatio;

    // Store position
    const pos: ItemPosition = { id: item.id, x, y, width: columnWidth, height };
    positions.set(item.id, pos);

    // Update column height
    columnHeights[minColumn] += height + gap;

    // Add to spatial grid
    addToSpatialGrid(item.id, y, y + height);
  }

  // Total height is the tallest column
  totalHeight = Math.max(...columnHeights);
}

/**
 * Masonry Horizontal Layout Algorithm (Flickr/Google Photos-style)
 * 
 * Fixed row heights, variable widths.
 * Groups items into rows, then justifies each row to fill the container width.
 * Creates a clean edge on both left and right sides.
 */
function calculateMasonryHorizontalLayout(): void {
  const { containerWidth, itemSize: targetRowHeight, gap } = config;

  let currentY = 0;
  let rowStart = 0;

  while (rowStart < items.length) {
    // Build a row by adding items until we exceed container width
    let rowWidth = 0;
    let rowEnd = rowStart;
    const rowItems: Array<{ item: LayoutItemInput; scaledWidth: number }> = [];

    while (rowEnd < items.length) {
      const item = items[rowEnd];
      const aspectRatio = item.aspectRatio > 0 ? item.aspectRatio : 1;
      // Width when scaled to target row height
      const scaledWidth = targetRowHeight * aspectRatio;
      const widthWithGap = scaledWidth + (rowItems.length > 0 ? gap : 0);

      // Check if adding this item would exceed container width
      // Allow at least one item per row
      if (rowItems.length > 0 && rowWidth + widthWithGap > containerWidth) {
        break;
      }

      rowItems.push({ item, scaledWidth });
      rowWidth += widthWithGap;
      rowEnd++;
    }

    // Calculate scale factor to justify row to container width
    // For the last row, don't stretch if it would make images too large
    const isLastRow = rowEnd >= items.length;
    const totalScaledWidth = rowItems.reduce((sum, r) => sum + r.scaledWidth, 0);
    const totalGaps = (rowItems.length - 1) * gap;
    const availableWidth = containerWidth - totalGaps;
    
    // Scale factor: how much to multiply each width to fill the row
    let scaleFactor = availableWidth / totalScaledWidth;
    
    // For last row, cap the scale factor to prevent oversized images
    if (isLastRow && scaleFactor > 1.2) {
      scaleFactor = 1.0; // Don't stretch last row
    }

    // Calculate actual row height based on scale
    const actualRowHeight = targetRowHeight * scaleFactor;

    // Position items in the row
    let currentX = 0;
    for (const { item, scaledWidth } of rowItems) {
      const width = scaledWidth * scaleFactor;
      const height = actualRowHeight;

      const pos: ItemPosition = { 
        id: item.id, 
        x: currentX, 
        y: currentY, 
        width, 
        height 
      };
      positions.set(item.id, pos);

      // Add to spatial grid
      addToSpatialGrid(item.id, currentY, currentY + height);

      currentX += width + gap;
    }

    currentY += actualRowHeight + gap;
    rowStart = rowEnd;
  }

  totalHeight = currentY;
}

/**
 * Grid Layout Algorithm
 * 
 * Places items in a uniform grid with fixed-size square cells.
 * Simple row/column calculation for O(1) position lookup.
 */
function calculateGridLayout(): void {
  const { containerWidth, itemSize, gap } = config;

  // Calculate columns that fit
  const columnCount = Math.max(1, Math.floor((containerWidth + gap) / (itemSize + gap)));
  
  // Adjust item size to fill container evenly
  const fittedSize = (containerWidth - (columnCount - 1) * gap) / columnCount;
  const rowHeight = fittedSize + gap;

  for (let i = 0; i < items.length; i++) {
    const item = items[i];
    const row = Math.floor(i / columnCount);
    const col = i % columnCount;

    const x = col * (fittedSize + gap);
    const y = row * rowHeight;

    const pos: ItemPosition = { id: item.id, x, y, width: fittedSize, height: fittedSize };
    positions.set(item.id, pos);

    // Add to spatial grid
    addToSpatialGrid(item.id, y, y + fittedSize);
  }

  // Total height based on number of rows
  const totalRows = Math.ceil(items.length / columnCount);
  totalHeight = totalRows * rowHeight;
}

// ============================================================================
// Spatial Grid Operations
// ============================================================================

/**
 * Adds an item to the spatial grid cells it occupies.
 * An item may span multiple cells if it's taller than CELL_HEIGHT.
 */
function addToSpatialGrid(id: number, startY: number, endY: number): void {
  const startCell = Math.floor(startY / CELL_HEIGHT);
  const endCell = Math.floor(endY / CELL_HEIGHT);

  for (let cell = startCell; cell <= endCell; cell++) {
    let cellItems = spatialGrid.get(cell);
    if (!cellItems) {
      cellItems = [];
      spatialGrid.set(cell, cellItems);
    }
    cellItems.push(id);
  }
}

/**
 * Queries visible items based on scroll position.
 * O(1) complexity thanks to spatial grid - only checks relevant cells.
 */
function handleScroll(scrollTop: number, viewportHeight: number): void {
  const { buffer } = config;

  const startY = Math.max(0, scrollTop - buffer);
  const endY = scrollTop + viewportHeight + buffer;

  const startCell = Math.floor(startY / CELL_HEIGHT);
  const endCell = Math.floor(endY / CELL_HEIGHT);

  // Collect unique IDs from all intersecting cells
  const visibleIds = new Set<number>();

  for (let cell = startCell; cell <= endCell; cell++) {
    const cellItems = spatialGrid.get(cell);
    if (cellItems) {
      for (const id of cellItems) {
        visibleIds.add(id);
      }
    }
  }

  // Map IDs to positions, with fine-grained Y check
  const visibleItems: ItemPosition[] = [];

  for (const id of visibleIds) {
    const pos = positions.get(id);
    if (pos) {
      // Double-check Y bounds (cell might include items just outside range)
      if (pos.y + pos.height > startY && pos.y < endY) {
        visibleItems.push(pos);
      }
    }
  }

  // Sort by Y for consistent render order (helps React/SolidJS keyed lists)
  visibleItems.sort((a, b) => a.y - b.y || a.x - b.x);

  respond({ type: "VISIBLE_UPDATE", payload: visibleItems });
}

// ============================================================================
// Utilities
// ============================================================================

function respond(message: WorkerOutMessage): void {
  self.postMessage(message);
}
