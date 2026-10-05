import { createMemo, Accessor } from 'solid-js';
import type { VirtualRange, ScrollAlignment } from '../types';

/**
 * Configuration parameters for the useVirtualList hook.
 *
 * @template TItem - The data record type for individual items in the list.
 */
export interface UseVirtualListOptions<TItem> {
    /**
     * Reactive accessor returning the complete collection of data items.
     */
    items: Accessor<TItem[]>;
    /**
     * Fixed item height in pixels or a resolver function calculating the height for a given item and index.
     * Can also be wrapped in a reactive accessor.
     */
    itemHeight:
        | Accessor<number | ((item: TItem, index: number) => number)>
        | number
        | ((item: TItem, index: number) => number);
    /**
     * Vertical spacing in pixels between consecutive items.
     * Defaults to 0.
     */
    gap?: Accessor<number> | number;
    /**
     * Number of buffer items rendered above and below the visible viewport to prevent blank spaces.
     * Defaults to 5.
     */
    overscanCount?: Accessor<number> | number;
    /**
     * Reactive accessor providing the current vertical scroll position of the viewport in pixels.
     */
    scrollOffset: Accessor<number>;
    /**
     * Reactive accessor providing the visible height of the container in pixels.
     */
    viewportHeight: Accessor<number>;
}

/**
 * Return signature of the useVirtualList hook.
 *
 * @template TItem - The data record type for individual items in the list.
 */
export interface UseVirtualListReturn<TItem> {
    /**
     * Reactive accessor providing the current rendered item boundary indices.
     */
    visibleRange: Accessor<VirtualRange>;
    /**
     * Reactive accessor providing the total scrollable height in pixels.
     */
    totalHeightInPixels: Accessor<number>;
    /**
     * Reactive accessor providing the sliced array of items currently selected for rendering.
     */
    renderedSliceOfItems: Accessor<TItem[]>;
    /**
     * Resolves the vertical offset in pixels from the top of the track for a specified item index.
     *
     * @param {number} itemIndex - The index of the item.
     * @returns {number} The vertical position in pixels.
     */
    getItemOffsetInPixels: (itemIndex: number) => number;
    /**
     * Resolves the height in pixels for a specified item index.
     *
     * @param {number} itemIndex - The index of the item.
     * @returns {number} The height in pixels.
     */
    getItemHeightInPixels: (itemIndex: number) => number;
    /**
     * Calculates the target scroll offset in pixels required to scroll an item index into view.
     *
     * @param {number} targetIndex - The index in the full data collection to target.
     * @param {ScrollAlignment} [alignment] - The alignment within the viewport ('start', 'center', 'end', or 'auto').
     * @returns {number} The target scroll position in pixels.
     */
    calculateScrollOffsetForIndex: (targetIndex: number, alignment?: ScrollAlignment) => number;
}

/**
 * Custom Solid.js hook computing windowed virtualization ranges and offsets for large collections.
 *
 * Supports both fixed item heights and dynamic functions, accounting for vertical gaps,
 * configurable overscan buffers, and precise viewport boundaries.
 *
 * @template TItem - The data record type for individual items in the list.
 * @param {UseVirtualListOptions<TItem>} options - The hook options and reactive accessors.
 * @returns {UseVirtualListReturn<TItem>} Reactive virtualization values and utility functions.
 *
 * @example
 * ```tsx
 * const { visibleRange, totalHeightInPixels, renderedSliceOfItems } = useVirtualList({
 *     items: () => dataList(),
 *     itemHeight: 150,
 *     gap: 8,
 *     overscanCount: 5,
 *     scrollOffset: currentScrollOffset,
 *     viewportHeight: containerViewportHeight
 * });
 * ```
 */
export function useVirtualList<TItem>(
    options: UseVirtualListOptions<TItem>
): UseVirtualListReturn<TItem> {
    const resolvedGap = createMemo(() => {
        if (typeof options.gap === 'function') {
            return options.gap();
        }
        return options.gap ?? 0;
    });

    const resolvedOverscanCount = createMemo(() => {
        if (typeof options.overscanCount === 'function') {
            return options.overscanCount();
        }
        return options.overscanCount ?? 5;
    });

    const resolvedItemHeightProp = createMemo(() => {
        if (typeof options.itemHeight === 'function' && options.itemHeight.length === 0) {
            return (
                options.itemHeight as () => number | ((item: TItem, index: number) => number)
            )();
        }
        return options.itemHeight;
    });

    const isFixedItemHeight = createMemo(() => typeof resolvedItemHeightProp() === 'number');

    /**
     * Memoized computation for variable height item collections.
     * Computes cumulative offset and individual height arrays for binary search lookups.
     */
    const variableHeightMetadata = createMemo(() => {
        if (isFixedItemHeight()) {
            return null;
        }

        const heightResolverFunction = resolvedItemHeightProp() as (
            item: TItem,
            index: number
        ) => number;
        const currentItemList = options.items();
        const totalItemCount = currentItemList.length;
        const currentGap = resolvedGap();

        const itemOffsetsArray: number[] = new Array(totalItemCount);
        const itemHeightsArray: number[] = new Array(totalItemCount);
        let cumulativeOffsetInPixels = 0;

        for (let itemIndex = 0; itemIndex < totalItemCount; itemIndex = itemIndex + 1) {
            const currentItem = currentItemList[itemIndex];
            const resolvedItemHeight = heightResolverFunction(currentItem, itemIndex);

            itemOffsetsArray[itemIndex] = cumulativeOffsetInPixels;
            itemHeightsArray[itemIndex] = resolvedItemHeight;

            cumulativeOffsetInPixels = cumulativeOffsetInPixels + resolvedItemHeight + currentGap;
        }

        const calculatedTotalHeight =
            totalItemCount > 0 ? cumulativeOffsetInPixels - currentGap : 0;

        return {
            itemOffsets: itemOffsetsArray,
            itemHeights: itemHeightsArray,
            totalHeightInPixels: Math.max(0, calculatedTotalHeight)
        };
    });

    /**
     * Resolves the height in pixels for an item at a given index.
     */
    const getItemHeightInPixels = (itemIndex: number): number => {
        if (isFixedItemHeight()) {
            return resolvedItemHeightProp() as number;
        }

        const metadata = variableHeightMetadata();
        if (!metadata || itemIndex < 0 || itemIndex >= metadata.itemHeights.length) {
            return 0;
        }

        return metadata.itemHeights[itemIndex] || 0;
    };

    /**
     * Resolves the vertical offset in pixels from the top of the track for an item at a given index.
     */
    const getItemOffsetInPixels = (itemIndex: number): number => {
        if (itemIndex <= 0) {
            return 0;
        }

        if (isFixedItemHeight()) {
            const fixedHeight = resolvedItemHeightProp() as number;
            const itemStepInPixels = fixedHeight + resolvedGap();
            return itemIndex * itemStepInPixels;
        }

        const metadata = variableHeightMetadata();
        if (!metadata || itemIndex >= metadata.itemOffsets.length) {
            return 0;
        }

        return metadata.itemOffsets[itemIndex] || 0;
    };

    /**
     * Memoized total height of the scrollable virtual track in pixels.
     */
    const totalHeightInPixels = createMemo(() => {
        const totalItemCount = options.items().length;
        if (totalItemCount === 0) {
            return 0;
        }

        if (isFixedItemHeight()) {
            const fixedHeight = resolvedItemHeightProp() as number;
            const itemStepInPixels = fixedHeight + resolvedGap();
            return Math.max(0, totalItemCount * itemStepInPixels - resolvedGap());
        }

        const metadata = variableHeightMetadata();
        return metadata ? metadata.totalHeightInPixels : 0;
    });

    /**
     * Memoized calculation of the start and end boundary indices for rendering.
     */
    const visibleRange = createMemo<VirtualRange>(() => {
        const totalItemCount = options.items().length;
        if (totalItemCount === 0) {
            return { startIndex: 0, endIndex: 0 };
        }

        const currentScrollOffset = Math.max(0, options.scrollOffset());
        const currentViewportHeight = Math.max(0, options.viewportHeight());
        const overscanCountValue = resolvedOverscanCount();

        if (isFixedItemHeight()) {
            const fixedHeight = resolvedItemHeightProp() as number;
            const itemStepInPixels = fixedHeight + resolvedGap();

            if (itemStepInPixels <= 0) {
                return { startIndex: 0, endIndex: totalItemCount };
            }

            const rawStartIndex = Math.floor(currentScrollOffset / itemStepInPixels);
            const visibleItemCount = Math.ceil(currentViewportHeight / itemStepInPixels);
            const rawEndIndex = rawStartIndex + visibleItemCount;

            const startIndex = Math.max(0, rawStartIndex - overscanCountValue);
            const endIndex = Math.min(totalItemCount, rawEndIndex + overscanCountValue);

            return { startIndex, endIndex };
        }

        const metadata = variableHeightMetadata();
        if (!metadata || metadata.itemOffsets.length === 0) {
            return { startIndex: 0, endIndex: 0 };
        }

        const itemOffsets = metadata.itemOffsets;
        const itemHeights = metadata.itemHeights;

        /** Binary search for the first item intersecting currentScrollOffset */
        let binarySearchLowIndex = 0;
        let binarySearchHighIndex = totalItemCount - 1;
        let rawStartIndex = 0;

        while (binarySearchLowIndex <= binarySearchHighIndex) {
            const middleIndex = Math.floor((binarySearchLowIndex + binarySearchHighIndex) / 2);
            const itemTopOffset = itemOffsets[middleIndex];
            const itemBottomOffset = itemTopOffset + itemHeights[middleIndex];

            if (itemBottomOffset >= currentScrollOffset) {
                rawStartIndex = middleIndex;
                binarySearchHighIndex = middleIndex - 1;
            } else {
                binarySearchLowIndex = middleIndex + 1;
            }
        }

        /** Linear forward scan to locate the raw end index within viewport */
        const viewportBottomThreshold = currentScrollOffset + currentViewportHeight;
        let rawEndIndex = rawStartIndex;

        while (rawEndIndex < totalItemCount && itemOffsets[rawEndIndex] < viewportBottomThreshold) {
            rawEndIndex = rawEndIndex + 1;
        }

        const startIndex = Math.max(0, rawStartIndex - overscanCountValue);
        const endIndex = Math.min(totalItemCount, rawEndIndex + overscanCountValue);

        return { startIndex, endIndex };
    });

    /**
     * Sliced subset of items prioritized for current DOM rendering.
     */
    const renderedSliceOfItems = createMemo(() => {
        const range = visibleRange();
        return options.items().slice(range.startIndex, range.endIndex);
    });

    /**
     * Calculates the target scroll offset in pixels required to scroll an item index into view.
     */
    const calculateScrollOffsetForIndex = (
        targetIndex: number,
        alignment: ScrollAlignment = 'auto'
    ): number => {
        const currentViewportHeight = options.viewportHeight();
        const currentScrollOffset = options.scrollOffset();
        const itemTopBoundary = getItemOffsetInPixels(targetIndex);
        const itemHeightValue = getItemHeightInPixels(targetIndex);
        const itemBottomBoundary = itemTopBoundary + itemHeightValue;

        if (alignment === 'start') {
            return itemTopBoundary;
        }

        if (alignment === 'end') {
            return Math.max(0, itemBottomBoundary - currentViewportHeight);
        }

        if (alignment === 'center') {
            return Math.max(0, itemTopBoundary - (currentViewportHeight - itemHeightValue) / 2);
        }

        /** Auto alignment: only scroll if the item is outside the visible viewport */
        if (itemTopBoundary < currentScrollOffset) {
            return itemTopBoundary;
        }

        if (itemBottomBoundary > currentScrollOffset + currentViewportHeight) {
            return Math.max(0, itemBottomBoundary - currentViewportHeight);
        }

        return currentScrollOffset;
    };

    return {
        visibleRange,
        totalHeightInPixels,
        renderedSliceOfItems,
        getItemOffsetInPixels,
        getItemHeightInPixels,
        calculateScrollOffsetForIndex
    };
}
