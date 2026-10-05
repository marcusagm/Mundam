import { JSX, Accessor } from 'solid-js';

/**
 * Representation of the currently visible and buffered item range in the virtual list.
 */
export interface VirtualRange {
    /**
     * The first index in the data collection rendered in the virtualized slice (inclusive).
     */
    startIndex: number;
    /**
     * The boundary index marking the end of the rendered virtualized slice (exclusive).
     */
    endIndex: number;
}

/**
 * Alignment options when programmatically scrolling to a specific item index.
 */
export type ScrollAlignment = 'start' | 'center' | 'end' | 'auto';

/**
 * Imperative controller handle exposing programmatic scrolling methods for the virtual list.
 */
export interface VirtualListController {
    /**
     * Scrolls the virtual container so the target item index is brought into view.
     *
     * @param {number} targetIndex - The index in the full data collection to scroll to.
     * @param {ScrollAlignment} [alignment] - Where to position the item within the viewport.
     */
    scrollToIndex: (targetIndex: number, alignment?: ScrollAlignment) => void;
    /**
     * Scrolls the virtual container to the very top.
     */
    scrollToTop: () => void;
    /**
     * Scrolls the virtual container to the very bottom.
     */
    scrollToBottom: () => void;
}

/**
 * Configuration properties for the generic VirtualList component.
 *
 * @template TItem - The data record type for individual items in the list.
 */
export interface VirtualListProperties<TItem> {
    /**
     * Complete array of items to render through windowed virtualization.
     */
    items: TItem[];
    /**
     * Fixed height in pixels per item, or a function resolving the height for an item and its index.
     */
    itemHeight: number | ((item: TItem, index: number) => number);
    /**
     * Vertical spacing in pixels between consecutive rendered items.
     * Defaults to 0.
     */
    gap?: number;
    /**
     * Number of buffer items rendered above and below the visible viewport to prevent blank flickers.
     * Defaults to 5.
     */
    overscanCount?: number;
    /**
     * Explicit height of the outer scroll container (e.g. '100%', 500, or '500px').
     * Defaults to '100%'.
     */
    containerHeight?: number | string;
    /**
     * Explicit width of the outer scroll container (e.g. '100%', 300, or '300px').
     * Defaults to '100%'.
     */
    containerWidth?: number | string;
    /**
     * Target index to automatically scroll into view when updated (e.g. active keyboard selection).
     */
    autoScrollToIndex?: number;
    /**
     * Additional CSS class names to apply to the outer scrollable container.
     */
    class?: string;
    /**
     * Inline styles applied to the outer scrollable container.
     */
    style?: JSX.CSSProperties | string;
    /**
     * ARIA role assigned to the scrollable container.
     * Defaults to 'list'.
     */
    listRole?: JSX.HTMLAttributes<HTMLDivElement>['role'];
    /**
     * ARIA role assigned to each virtual item wrapper.
     * Defaults to 'listitem'.
     */
    itemRole?: JSX.HTMLAttributes<HTMLDivElement>['role'];
    /**
     * Accessible label describing the virtual list content for assistive technologies.
     */
    ariaLabel?: string;
    /**
     * Unique property field or key extractor function used to identify items in the list.
     */
    keyField?: keyof TItem | ((item: TItem, index: number) => string | number);
    /**
     * Callback invoked whenever a scroll event occurs on the container.
     */
    onScroll?: (event: Event) => void;
    /**
     * Callback invoked whenever the rendered visible range indices change.
     */
    onVisibleRangeChange?: (visibleRange: VirtualRange) => void;
    /**
     * Callback invoked whenever the slice of rendered items changes.
     * Frequently used for preloading assets or thumbnails of items entering the viewport.
     */
    onVisibleItemsChange?: (visibleItemList: TItem[]) => void;
    /**
     * Callback receiving the imperative controller handle for programmatic scrolling.
     */
    controller?: (controllerHandle: VirtualListController) => void;
    /**
     * Fallback element rendered when the items array is empty.
     */
    fallback?: JSX.Element;
    /**
     * Render prop function invoked for each visible item.
     * Receives the item data and a reactive accessor for its global index.
     */
    children: (item: TItem, indexAccessor: Accessor<number>) => JSX.Element;
}
