import { createSignal, onMount, onCleanup, For, createMemo, Show, untrack } from 'solid-js';
import { AssetCard } from '../assets/AssetCard';
import { EmptyState } from '../components/EmptyState';
import { AssetContextMenu } from '../components/AssetContextMenu';
import { type AssetItem } from '../../../../types';
import {
    useLibrary,
    useAssetCardActions,
    useVirtualViewport,
    toLayoutItems,
    useGridKeyboardNav,
    useSelection
} from '../../../../core/hooks';
import { scheduler } from '../../../../core/utils/scheduler';
import '../viewport.css';

/**
 * Scroll load more threshold
 *
 * @type {number}
 */
const SCROLL_LOAD_MORE_THRESHOLD = 500;

/**
 * Resize debounce threshold
 *
 * @type {number}
 */
const RESIZE_DEBOUNCE_THRESHOLD = 5;

/**
 * Fallback sidebar width
 *
 * @type {number}
 */
const FALLBACK_SIDEBAR_WIDTH = 80;

/**
 * VirtualMasonryProps - Input data
 *
 * @interface VirtualMasonryProps
 */
interface VirtualMasonryProps {
    items: AssetItem[];
    mode?: 'masonry-v' | 'masonry-h';
    gap?: number;
    buffer?: number;
}

/**
 * VirtualMasonry - Worker-based Virtualized Masonry Layout
 *
 * Uses a Web Worker for layout calculations and Spatial Grid for O(1) visibility queries.
 *
 * @param {VirtualMasonryProps} props - Input data
 * @returns {JSX.Element} Structural component
 */
export function VirtualMasonry(props: VirtualMasonryProps) {
    /**
     * Library hook
     *
     * @type {LibraryStore}
     */
    const library = useLibrary();

    /**
     * Asset card actions hook
     *
     * @type {AssetCardActionsStore}
     */
    const actions = useAssetCardActions();

    /**
     * Selection hook
     *
     * @type {SelectionStore}
     */
    const selection = useSelection();

    /**
     * Scroll container signal
     *
     * @type {Signal<HTMLDivElement>}
     */
    const [scrollContainer, setScrollContainer] = createSignal<HTMLDivElement>();

    /**
     * Container height signal
     *
     * @type {Signal<number>}
     */
    const [containerHeight, setContainerHeight] = createSignal(0);

    /**
     * Context menu open signal
     *
     * @type {Signal<boolean>} - Context menu open
     */
    const [contextMenuOpen, setContextMenuOpen] = createSignal(false);

    /**
     * Context menu position signal
     *
     * @type {Signal<{ coordinateX: number; coordinateY: number }>} - Context menu position
     */
    const [contextMenuPosition, setContextMenuPosition] = createSignal({
        coordinateX: 0,
        coordinateY: 0
    });

    /**
     * Context menu asset signal
     *
     * @type {Signal<AssetItem | null>} - Context menu asset
     */
    const [contextMenuAsset, setContextMenuAsset] = createSignal<AssetItem | null>(null);

    /**
     * Handle context menu
     *
     * @param {MouseEvent} event - Mouse event
     * @param {string} id - Asset ID
     */
    const handleContextMenu = (event: MouseEvent, id: string) => {
        event.preventDefault();
        const item = itemsById().get(id);
        if (item) {
            setContextMenuAsset(item);
            setContextMenuPosition({ coordinateX: event.clientX, coordinateY: event.clientY });
            setContextMenuOpen(true);
        }
    };

    /**
     * Convert items to Worker-friendly format (minimal data)
     *
     * @type {Memo<AssetItem[]>}
     */
    const layoutItems = createMemo(() => toLayoutItems(props.items));

    /**
     * Connect to the layout Worker with the specified mode
     *
     * @type {VirtualViewportStore}
     */
    const layoutMode = () => props.mode || 'masonry-v';

    /**
     * Virtual viewport hook
     *
     * @type {VirtualViewportStore}
     */
    const viewport = useVirtualViewport(layoutMode, () => layoutItems(), {
        get gap() {
            return props.gap;
        },
        get buffer() {
            return props.buffer;
        }
    });

    /**
     * Items by ID memo
     *
     * @type {Memo<Map<number, AssetItem>>}
     */
    const itemsById = createMemo(() => {
        const map = new Map<string, AssetItem>();
        props.items.forEach(item => map.set(item.id, item));
        return map;
    });

    /**
     * Keyboard navigation hook
     *
     * @type {GridKeyboardNavStore}
     */
    const keyboardNav = useGridKeyboardNav({
        visibleItems: viewport.visibleItems,
        allItems: () => props.items,
        containerHeight,
        scrollContainer,
        onSelect: (itemId: string, modifiers: { multi: boolean; shift: boolean }) =>
            actions.handleSelect(itemId, modifiers),
        onSelectAll: () => selection.select(library.items.map(item => item.id)),
        onDeselectAll: selection.clear,
        onMoveToTrash: actions.handleMoveToTrash,
        onOpen: (itemId: string) => actions.handleOpen(itemId),
        isSelected: actions.isSelected,
        getSelectedIds: actions.getSelectedIds,
        getItemRect: itemId => viewport.getItemPosition(itemId)
    });

    /**
     * Handle select with focus
     *
     * @param {number} itemId - ID of the item to select
     * @param {{ multi: boolean; shift: boolean }} modifiers - Modifiers for the selection
     */
    const handleSelectWithFocus = (
        itemId: string,
        modifiers: { multi: boolean; shift: boolean }
    ) => {
        keyboardNav.syncFocusWithClick(itemId);
        actions.handleSelect(itemId, modifiers);
    };

    /**
     * Get item info
     *
     * @param {number} itemId - ID of the item
     * @returns {AssetItem | undefined} Item object or undefined
     */
    const getItemInfo = (itemId: string) => {
        const item = itemsById().get(itemId);
        if (!item) return undefined;
        return {
            path: item.path,
            thumbnail_path: item.thumbnail_path
        };
    };

    /**
     * Last reported width
     *
     * @type {number}
     */
    let lastReportedWidth = 0;

    /**
     * On mount
     */
    onMount(() => {
        const element = scrollContainer();
        if (!element) return;

        const observer = new ResizeObserver(entries => {
            scheduler.schedule(() => {
                const entry = entries[0];
                if (!entry) return;

                const width = entry.contentRect.width;
                const height = entry.contentRect.height;

                setContainerHeight(height);

                if (width > 0 && Math.abs(width - lastReportedWidth) > RESIZE_DEBOUNCE_THRESHOLD) {
                    lastReportedWidth = width;
                    viewport.handleResize(width);
                }
            });
        });

        observer.observe(element);

        const initialRect = element.getBoundingClientRect();
        if (initialRect.width > 0) {
            setContainerHeight(initialRect.height);
            lastReportedWidth = initialRect.width;
            viewport.handleResize(initialRect.width);
        } else {
            const estimatedWidth = window.innerWidth - FALLBACK_SIDEBAR_WIDTH;
            if (estimatedWidth > 0) {
                lastReportedWidth = estimatedWidth;
                viewport.handleResize(estimatedWidth);
            }
        }

        let isScrollScheduled = false;

        const handleScroll = () => {
            if (isScrollScheduled) return;
            isScrollScheduled = true;

            const currentContainerHeight = untrack(() => containerHeight());

            scheduler.schedule(() =>
                untrack(() => {
                    isScrollScheduled = false;
                    const containerElement = scrollContainer();
                    if (!containerElement) return;

                    viewport.handleScroll(containerElement.scrollTop, currentContainerHeight);

                    const { scrollTop, scrollHeight, clientHeight } = containerElement;
                    if (scrollTop + clientHeight >= scrollHeight - SCROLL_LOAD_MORE_THRESHOLD) {
                        library.loadMore();
                    }
                })
            );
        };

        element.addEventListener('scroll', handleScroll, { passive: true });
        viewport.handleScroll(0, containerHeight());

        onCleanup(() => {
            observer.disconnect();
            element.removeEventListener('scroll', handleScroll);
        });
    });

    return (
        <div
            ref={setScrollContainer}
            class="virtual-scroll-container"
            role="grid"
            aria-label="Asset gallery - masonry layout"
            tabIndex={0}
        >
            <Show
                when={props.items.length > 0}
                fallback={
                    <EmptyState
                        title="No assets found"
                        description="Try adjusting your filters or add assets to your library."
                    />
                }
            >
                <div
                    class="virtual-track"
                    role="rowgroup"
                    style={{
                        height: `${viewport.totalHeight()}px`,
                        position: 'relative'
                    }}
                >
                    <For each={viewport.visibleItems()}>
                        {position => {
                            const item = itemsById().get(position.id);
                            if (!item) return null;

                            const isFocused = () => keyboardNav.focusedId() === item.id;

                            return (
                                <AssetCard
                                    item={item}
                                    isSelected={selection.isItemSelected(item.id)}
                                    isFocused={isFocused()}
                                    style={{
                                        position: 'absolute',
                                        transform: `translate3d(${position.x}px, ${position.y}px, 0)`,
                                        width: `${position.width}px`,
                                        height: `${position.height}px`
                                    }}
                                    onSelect={handleSelectWithFocus}
                                    onOpen={actions.handleOpen}
                                    onContextMenu={handleContextMenu}
                                    getSelectedIds={actions.getSelectedIds}
                                    getItemInfo={getItemInfo}
                                />
                            );
                        }}
                    </For>
                </div>
            </Show>

            <AssetContextMenu
                isOpen={contextMenuOpen()}
                coordinateX={contextMenuPosition().coordinateX}
                coordinateY={contextMenuPosition().coordinateY}
                asset={contextMenuAsset()}
                onClose={() => setContextMenuOpen(false)}
            />
        </div>
    );
}
