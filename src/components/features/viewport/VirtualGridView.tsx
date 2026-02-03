import { Component, createSignal, onMount, onCleanup, For, createMemo, Show } from "solid-js";
import { AssetCard } from "./AssetCard";
import { EmptyState } from "./EmptyState";
import {
  useLibrary,
  useAssetCardActions,
  useVirtualViewport,
  useGridKeyboardNav,
  useSelection,
} from "../../../core/hooks";
import type { LayoutItemInput } from "../../../core/viewport";
import "./grid-view.css";

/**
 * VirtualGridView - Worker-based Virtualized Grid Layout
 *
 * Uses a Web Worker for layout calculations and Spatial Grid for O(1) visibility queries.
 * Grid layout uses uniform square cells, so aspectRatio is always 1.
 * 
 * Features:
 * - Keyboard navigation (Arrow keys, Home, End)
 * - Scroll-to-focus when navigating
 * - Space to select, Enter to open
 */
export const VirtualGridView: Component = () => {
  const lib = useLibrary();
  const actions = useAssetCardActions();
  const selection = useSelection();

  const [scrollContainer, setScrollContainer] = createSignal<HTMLDivElement>();

  const [_containerWidth, setContainerWidth] = createSignal(0);
  const [containerHeight, setContainerHeight] = createSignal(0);

  // For grid, all items have aspectRatio = 1 (square cells)
  const layoutItems = createMemo((): LayoutItemInput[] =>
    lib.items.map((item) => ({
      id: item.id,
      aspectRatio: 1, // Grid uses uniform squares
    }))
  );

  // Connect to the layout Worker in grid mode
  const viewport = useVirtualViewport("grid", layoutItems);

  // Create item lookup Map for O(1) access during render
  const itemsById = createMemo(() => {
    const map = new Map<number, (typeof lib.items)[0]>();
    lib.items.forEach((item) => map.set(item.id, item));
    return map;
  });

  // Keyboard navigation
  const keyboardNav = useGridKeyboardNav({
    visibleItems: viewport.visibleItems,
    allItems: () => lib.items,
    containerHeight,
    scrollContainer, // Pass accessor
    onSelect: (id: number, multi: boolean) => selection.toggle(id, multi),
    onOpen: (id) => actions.handleOpen(id),
    isSelected: actions.isSelected,
    getSelectedIds: actions.getSelectedIds,
    getItemRect: (id) => viewport.getItemPosition(id),
  });

  // Wrap select to sync focus
  const handleSelectWithFocus = (id: number, multi: boolean) => {
    keyboardNav.syncFocusWithClick(id);
    actions.handleSelect(id, multi);
  };

  // DnD helper: get item info by ID (used by drag source for ghost creation)
  const getItemInfo = (id: number) => {
    const item = itemsById().get(id);
    if (!item) return undefined;
    return {
      path: item.path,
      thumbnail_path: item.thumbnail_path,
    };
  };

  // Track last width sent to Worker to prevent loops
  let lastReportedWidth = 0;

  onMount(() => {
    const el = scrollContainer();
    if (!el) return;

    const observer = new ResizeObserver((entries) => {
      requestAnimationFrame(() => {
        const entry = entries[0];
        if (!entry) return;

        const width = entry.contentRect.width;
        const height = entry.contentRect.height;

        setContainerHeight(height);
        
        // Only resize if change is significant (> 5px)
        // This prevents loops caused by scrollbar appearing/disappearing
        if (width > 0 && Math.abs(width - lastReportedWidth) > 5) {
          setContainerWidth(width);
          lastReportedWidth = width;
          viewport.handleResize(width);
        }
      });
    });

    observer.observe(el);

    // Initial measure
    const rect = el.getBoundingClientRect();
    if (rect.width > 0) {
      setContainerWidth(rect.width);
      setContainerHeight(rect.height);
      lastReportedWidth = rect.width;
      viewport.handleResize(rect.width);
    }

    const handleScroll = () => {
      const el = scrollContainer();
      if (!el) return;
      const currentScrollTop = el.scrollTop;

      // Notify Worker of scroll position
      viewport.handleScroll(currentScrollTop, containerHeight());

      // Load more when near bottom
      const { scrollTop, scrollHeight, clientHeight } = el;
      if (
        scrollTop + clientHeight >=
        scrollHeight - 500
      ) {
        lib.loadMore();
      }
    };

    el.addEventListener("scroll", handleScroll, { passive: true });

    // Initial scroll notification
    viewport.handleScroll(0, containerHeight());

    onCleanup(() => {
      observer.disconnect();
      el.removeEventListener("scroll", handleScroll);
    });
  });

  return (
    <div 
      ref={setScrollContainer} 
      class="grid-view-container"
      role="grid"
      aria-label="Image gallery - grid layout"
      tabIndex={0}
    >
      <Show when={lib.items.length > 0} fallback={
        <EmptyState 
          title="No images found"
          description="Try adjusting your filters or add images to your library."
        />
      }>
      <div
        class="grid-view-track"
        role="rowgroup"
        style={{
          height: `${viewport.totalHeight()}px`,
          position: "relative",
        }}
      >
        <For each={viewport.visibleItems()}>
          {(pos) => {
            // O(1) lookup from our Map
            const item = itemsById().get(pos.id);
            if (!item) return null;

            const isFocused = () => keyboardNav.focusedId() === item.id;

            return (
              <AssetCard
                // Identity
                id={item.id}
                filename={item.filename}
                path={item.path}
                // Display
                thumbnailPath={item.thumbnail_path}
                width={item.width}
                height={item.height}
                // State
                isSelected={actions.isSelected(item.id)}
                isFocused={isFocused()}
                style={{
                  position: "absolute",
                  top: 0,
                  left: 0,
                  transform: `translate3d(${pos.x}px, ${pos.y}px, 0)`,
                  width: `${pos.width}px`,
                  height: `${pos.height}px`,
                }}
                // Callbacks
                onSelect={handleSelectWithFocus}
                onOpen={actions.handleOpen}
                // DnD - drag source only (drop handled at container level)
                getSelectedIds={actions.getSelectedIds}
                getItemInfo={getItemInfo}
              />
            );
          }}
        </For>
      </div>
      </Show>
    </div>
  );
};
