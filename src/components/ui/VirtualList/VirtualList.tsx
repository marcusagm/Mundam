import {
    createSignal,
    createMemo,
    createEffect,
    on,
    untrack,
    onMount,
    onCleanup,
    For,
    Show,
    splitProps,
    JSX
} from 'solid-js';
import { cn } from '../../../lib/utils';
import { useVirtualList } from './hooks/useVirtualList';
import type { VirtualListProperties, ScrollAlignment, VirtualListController } from './types';
import './virtual-list.css';

/**
 * High-performance virtualized list component for Solid.js.
 *
 * Renders only the items that fit in the visible viewport plus configurable buffer overscan.
 * Drastically reduces DOM nodes and memory footprint for large datasets, enabling efficient
 * thumbnail deck preloading, keyboard navigation, and smooth scrolling.
 *
 * @template TItem - The data record type for individual items in the list.
 * @param {VirtualListProperties<TItem>} componentProperties - Configuration properties for the virtual list.
 * @returns {JSX.Element} The rendered virtual list container and track.
 *
 * @example
 * ```tsx
 * import { VirtualList } from '@/components/ui';
 *
 * <VirtualList
 *     items={duplicateGroups()}
 *     itemHeight={150}
 *     gap={8}
 *     overscanCount={5}
 *     onVisibleItemsChange={(visibleGroups) => preloadGroupThumbnails(visibleGroups)}
 *     keyField="id"
 * >
 *     {(groupItem, indexAccessor) => (
 *         <DuplicateGroupItem group={groupItem} index={indexAccessor()} />
 *     )}
 * </VirtualList>
 * ```
 */
export function VirtualList<TItem>(componentProperties: VirtualListProperties<TItem>): JSX.Element {
    const [localProperties] = splitProps(componentProperties, [
        'items',
        'itemHeight',
        'gap',
        'overscanCount',
        'containerHeight',
        'containerWidth',
        'autoScrollToIndex',
        'class',
        'style',
        'listRole',
        'itemRole',
        'ariaLabel',
        'keyField',
        'onScroll',
        'onVisibleRangeChange',
        'onVisibleItemsChange',
        'controller',
        'fallback',
        'children'
    ]);

    /** Reference to the outer scrollable DOM element */
    let scrollContainerReference: HTMLDivElement | undefined;

    /** Reactive vertical scroll offset in pixels */
    const [scrollOffset, setScrollOffset] = createSignal(0);

    /** Measured viewport height of the container in pixels */
    const [viewportHeight, setViewportHeight] = createSignal(0);

    /** Tracks the previous auto-scroll target index to prevent redundant scroll triggers */
    let previousAutoScrollIndex: number | undefined;

    /** Setup resize observer to dynamically measure container height */
    onMount(() => {
        if (!scrollContainerReference) {
            return;
        }

        setViewportHeight(scrollContainerReference.clientHeight);

        if (typeof ResizeObserver !== 'undefined') {
            const resizeObserver = new ResizeObserver(entries => {
                const firstEntry = entries[0];
                if (firstEntry) {
                    setViewportHeight(firstEntry.contentRect.height);
                }
            });

            resizeObserver.observe(scrollContainerReference);

            onCleanup(() => {
                resizeObserver.disconnect();
            });
        }
    });

    /** Virtualization engine calculations */
    const {
        visibleRange,
        totalHeightInPixels,
        renderedSliceOfItems,
        getItemOffsetInPixels,
        getItemHeightInPixels,
        calculateScrollOffsetForIndex
    } = useVirtualList({
        items: () => localProperties.items,
        itemHeight: () => localProperties.itemHeight,
        gap: () => localProperties.gap ?? 0,
        overscanCount: () => localProperties.overscanCount ?? 5,
        scrollOffset,
        viewportHeight
    });

    /**
     * Programmatically scrolls the container to position a specific item index into view.
     *
     * @param {number} targetIndex - Target item index.
     * @param {ScrollAlignment} [alignment] - Alignment rule within viewport.
     */
    const handleScrollToIndex = (
        targetIndex: number,
        alignment: ScrollAlignment = 'auto'
    ): void => {
        if (!scrollContainerReference) {
            return;
        }

        const calculatedTargetScrollTop = calculateScrollOffsetForIndex(targetIndex, alignment);
        scrollContainerReference.scrollTo({
            top: calculatedTargetScrollTop,
            behavior: 'auto'
        });
        setScrollOffset(calculatedTargetScrollTop);
    };

    /** Scrolls the container to the top */
    const handleScrollToTop = (): void => {
        if (!scrollContainerReference) {
            return;
        }
        scrollContainerReference.scrollTo({ top: 0, behavior: 'auto' });
        setScrollOffset(0);
    };

    /** Scrolls the container to the bottom */
    const handleScrollToBottom = (): void => {
        if (!scrollContainerReference) {
            return;
        }
        const maximumScrollTop = Math.max(0, totalHeightInPixels() - viewportHeight());
        scrollContainerReference.scrollTo({
            top: maximumScrollTop,
            behavior: 'auto'
        });
        setScrollOffset(maximumScrollTop);
    };

    /** Expose imperative controller methods if callback was supplied */
    createEffect(() => {
        const notifyController = localProperties.controller;
        if (notifyController) {
            const controllerHandle: VirtualListController = {
                scrollToIndex: handleScrollToIndex,
                scrollToTop: handleScrollToTop,
                scrollToBottom: handleScrollToBottom
            };
            notifyController(controllerHandle);
        }
    });

    /**
     * Reactively scrolls to target index only when autoScrollToIndex explicitly changes to a different index.
     * Uses untrack to avoid subscribing to scrollOffset or viewportHeight changes during scrolling.
     */
    createEffect(
        on(
            () => localProperties.autoScrollToIndex,
            targetIndex => {
                if (
                    typeof targetIndex === 'number' &&
                    targetIndex >= 0 &&
                    targetIndex !== previousAutoScrollIndex
                ) {
                    previousAutoScrollIndex = targetIndex;
                    untrack(() => {
                        handleScrollToIndex(targetIndex, 'auto');
                    });
                }
            },
            { defer: true }
        )
    );

    /**
     * Handles scroll events directly on every scroll event for instant, synchronous virtual window updates.
     */
    const handleContainerScroll = (event: Event): void => {
        const scrollableTarget = event.currentTarget as HTMLDivElement;
        setScrollOffset(scrollableTarget.scrollTop);
        localProperties.onScroll?.(event);
    };

    /** Immediately announces visible range changes */
    createEffect(
        on(
            () => visibleRange(),
            currentRange => {
                localProperties.onVisibleRangeChange?.(currentRange);
            }
        )
    );

    /**
     * Debounced announcement of visible items to throttle heavy operations
     * such as preloading thumbnail decks or triggering network requests.
     * Only triggers when visibleRange changes, untracking items array mutations.
     */
    createEffect(
        on(
            () => visibleRange(),
            currentRange => {
                const notifyVisibleItemsChange = localProperties.onVisibleItemsChange;
                if (!notifyVisibleItemsChange) {
                    return;
                }

                const currentItems = untrack(() => localProperties.items);
                if (!currentItems || currentItems.length === 0) {
                    return;
                }

                const currentlyVisibleItems = currentItems.slice(
                    currentRange.startIndex,
                    currentRange.endIndex
                );

                const debounceTimer = setTimeout(() => {
                    notifyVisibleItemsChange(currentlyVisibleItems);
                }, 150);

                onCleanup(() => {
                    clearTimeout(debounceTimer);
                });
            }
        )
    );

    return (
        <div
            ref={scrollContainerReference}
            class={cn('ui-virtual-list-container', localProperties.class)}
            style={{
                height:
                    typeof localProperties.containerHeight === 'number'
                        ? `${localProperties.containerHeight}px`
                        : (localProperties.containerHeight ?? '100%'),
                width:
                    typeof localProperties.containerWidth === 'number'
                        ? `${localProperties.containerWidth}px`
                        : (localProperties.containerWidth ?? '100%'),
                ...(typeof localProperties.style === 'object' ? localProperties.style : {})
            }}
            onScroll={handleContainerScroll}
            role={localProperties.listRole || 'list'}
            aria-label={localProperties.ariaLabel || 'Virtual List'}
            aria-setsize={localProperties.items.length}
            tabIndex={0}
        >
            <div
                class="ui-virtual-list-track"
                style={{
                    height: `${totalHeightInPixels()}px`
                }}
                role="presentation"
            >
                <Show when={localProperties.items.length > 0} fallback={localProperties.fallback}>
                    <For each={renderedSliceOfItems()}>
                        {(item, sliceIndexAccessor) => {
                            const resolvedGlobalIndex = createMemo(
                                () => visibleRange().startIndex + sliceIndexAccessor()
                            );
                            const resolvedItemOffset = createMemo(() =>
                                getItemOffsetInPixels(resolvedGlobalIndex())
                            );
                            const resolvedItemHeight = createMemo(() =>
                                getItemHeightInPixels(resolvedGlobalIndex())
                            );

                            return (
                                <div
                                    class="ui-virtual-list-item"
                                    style={{
                                        height: `${resolvedItemHeight()}px`,
                                        transform: `translate3d(0, ${resolvedItemOffset()}px, 0)`
                                    }}
                                    role={localProperties.itemRole || 'listitem'}
                                    aria-posinset={resolvedGlobalIndex() + 1}
                                >
                                    {localProperties.children(item, resolvedGlobalIndex)}
                                </div>
                            );
                        }}
                    </For>
                </Show>
            </div>
        </div>
    );
}
