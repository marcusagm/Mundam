import { type ImageItem } from "../types";

export interface LayoutOptions {
  items: ImageItem[];
  columns: number;
  containerWidth: number;
  gap: number;
}

export interface LayoutResult {
  height: number;
  positions: Map<number, { x: number; y: number; width: number; height: number }>;
}

export function calculateMasonryLayout(options: LayoutOptions): LayoutResult {
  const { items, columns, containerWidth, gap } = options;

  if (containerWidth <= 0 || columns <= 0 || items.length === 0) {
    return { height: 0, positions: new Map() };
  }

  const totalGapWidth = (columns - 1) * gap;
  // Ensure we don't have negative column width
  const colWidth = Math.max(0, (containerWidth - totalGapWidth) / columns);

  const colHeights = new Array(columns).fill(0);
  const positions = new Map();

  items.forEach((item) => {
    // Find the shortest column
    let minH = colHeights[0];
    let colIdx = 0;
    for (let i = 1; i < columns; i++) {
      if (colHeights[i] < minH) {
        minH = colHeights[i];
        colIdx = i;
      }
    }

    const x = colIdx * (colWidth + gap);
    const y = minH;

    // Default aspect ratio 1 if missing
    const aspectRatio = (item.width && item.height && item.height > 0) 
      ? item.width / item.height 
      : 1;
    
    // Height determined by column width and aspect ratio
    const h = colWidth / aspectRatio;

    positions.set(item.id, { x, y, width: colWidth, height: h });

    colHeights[colIdx] += h + gap;
  });

  const maxHeight = Math.max(...colHeights);

  return { height: maxHeight, positions };
}
