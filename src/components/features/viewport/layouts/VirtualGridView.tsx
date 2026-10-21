import {
    Component,
    createSignal,
    onMount,
    onCleanup,
    For,
    createMemo,
    Show,
    untrack,
    JSX
} from 'solid-js';
import { AssetCard } from '../assets/AssetCard';
import { EmptyState } from '../components/EmptyState';
import { AssetContextMenu } from '../components/AssetContextMenu';
import { AssetItem } from '../../../../types';
import {
    useLibrary,
    useAssetCardActions,
    useVirtualViewport,
    useGridKeyboardNav,
    useSelection
} from '../../../../core/hooks';
import { scheduler } from '../../../../core/utils/scheduler';
import type { LayoutItemInput } from '../../../../core/viewport';
import './grid-view.css';

/**
 * Load more threshold for scroll events
 *
 * @type {number}
 */
const SCROLL_LOAD_MORE_THRESHOLD = 500;

/**
 * Debounce threshold for resize events
 *
 * @type {number}
 */
const RESIZE_DEBOUNCE_THRESHOLD = 5;

/**
 * VirtualGridView - Worker-based Virtualized Grid Layout
 *
 * Uses a Web Worker for layout calculations and Spatial Grid for O(1) visibility queries.
 * Grid layout uses uniform square cells, so aspectRatio is always 1.
 *
 * @returns {JSX.Element} The virtual grid view component
 */
export const VirtualGridView: Component = (): JSX.Element => {
    /**
     * Library instance
     *
     * @type {Library}
     */
    const library = useLibrary();

    /**
     * Asset card actions
     *
     * @type {AssetCardActions}
     */
    const actions = useAssetCardActions();

    /**
     * Selection instance
     *
     * @type {Selection}
     */
    const selection = useSelection();

    /**
     * Scroll container ref
     *
     * @type {HTMLDivElement}
     */
    const [scrollContainer, setScrollContainer] = createSignal<HTMLDivElement>();

    /**
     * Container height
     *
     * @type {number}
     */
    const [containerHeight, setContainerHeight] = createSignal(0);

    const [contextMenuOpen, setContextMenuOpen] = createSignal(false);
    const [contextMenuPosition, setContextMenuPosition] = createSignal({
        coordinateX: 0,
        coordinateY: 0
    });
    const [contextMenuAsset, setContextMenuAsset] = createSignal<AssetItem | null>(null);

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
     * For grid, all items have aspectRatio = 1 (square cells)
     *
     * @type {LayoutItemInput[]}
     */
    const layoutItems = createMemo((): LayoutItemInput[] =>
        library.items.map(item => ({
            id: item.id,
            aspectRatio: 1
        }))
    );

    /**
     * Connect to the layout Worker in grid mode
     *
     * @type {VirtualViewport}
     */
    const viewport = useVirtualViewport('grid', () => layoutItems());

    /**
     * Items by ID
     *
     * @type {Map<number, AssetItem>}
     */
    const itemsById = createMemo(() => {
        const map = new Map<string, (typeof library.items)[0]>();
        library.items.forEach(item => map.set(item.id, item));
        return map;
    });

    /**
     * Keyboard navigation instance
     *
     * @type {GridKeyboardNav}
     */
    const keyboardNav = useGridKeyboardNav({
        visibleItems: viewport.visibleItems,
        allItems: () => library.items,
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
     * @param {number} itemId
     * @param {{ multi: boolean; shift: boolean }} modifiers
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
     * @param {number} itemId
     * @returns {AssetItem | undefined}
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
            class="grid-view-container"
            role="grid"
            aria-label="Assets gallery - grid layout"
            tabIndex={0}
        >
            <Show
                when={library.items.length > 0}
                fallback={
                    <EmptyState
                        title="No assets found"
                        description="Try adjusting your filters or add assets to your library."
                    />
                }
            >
                <div
                    class="grid-view-track"
                    role="rowgroup"
                    style={{
                        height: `${viewport.totalHeight()}px`,
                        position: 'relative'
                    }}
                >
                    <For each={viewport.visibleItems()}>
                        {position => {
                            const item = itemsById().get(position.id) as AssetItem | undefined;
                            if (!item) return null;

                            const isFocused = () => keyboardNav.focusedId() === item.id;

                            return (
                                <AssetCard
                                    item={item}
                                    isSelected={selection.isItemSelected(item.id)}
                                    isFocused={isFocused()}
                                    style={{
                                        position: 'absolute',
                                        top: 0,
                                        left: 0,
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
};
