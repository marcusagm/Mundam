import { Component, createSignal, onMount, onCleanup, For, createMemo } from "solid-js";
import { AssetCard } from "./AssetCard";
import {
  useLibrary,
  useAssetCardActions,
  useVirtualViewport,
} from "../../../core/hooks";
import type { LayoutItemInput } from "../../../core/viewport";
import "./grid-view.css";

/**
 * VirtualGridView - Worker-based Virtualized Grid Layout
 *
 * Uses a Web Worker for layout calculations and Spatial Grid for O(1) visibility queries.
 * Grid layout uses uniform square cells, so aspectRatio is always 1.
 */
export const VirtualGridView: Component = () => {
  const lib = useLibrary();
  const actions = useAssetCardActions();

  let scrollContainer: HTMLDivElement | undefined;

  const [containerWidth, setContainerWidth] = createSignal(0);
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

  // DnD helper: get item info by ID (used by drag source for ghost creation)
  const getItemInfo = (id: number) => {
    const item = itemsById().get(id);
    if (!item) return undefined;
    return {
      path: item.path,
      thumbnail_path: item.thumbnail_path,
    };
  };

  onMount(() => {
    if (!scrollContainer) return;

    const observer = new ResizeObserver((entries) => {
      requestAnimationFrame(() => {
        const entry = entries[0];
        if (!entry) return;

        const width = entry.contentRect.width;
        const height = entry.contentRect.height;

        setContainerHeight(height);
        if (width > 0 && Math.abs(width - containerWidth()) > 1) {
          setContainerWidth(width);
          viewport.handleResize(width);
        }
      });
    });

    observer.observe(scrollContainer);

    // Initial measure
    const rect = scrollContainer.getBoundingClientRect();
    if (rect.width > 0) {
      setContainerWidth(rect.width);
      setContainerHeight(rect.height);
      viewport.handleResize(rect.width);
    }

    const handleScroll = () => {
      if (!scrollContainer) return;
      const currentScrollTop = scrollContainer.scrollTop;

      // Notify Worker of scroll position
      viewport.handleScroll(currentScrollTop, containerHeight());

      // Load more when near bottom
      if (
        scrollContainer.scrollTop + scrollContainer.clientHeight >=
        scrollContainer.scrollHeight - 500
      ) {
        lib.loadMore();
      }
    };

    scrollContainer.addEventListener("scroll", handleScroll, { passive: true });

    // Initial scroll notification
    viewport.handleScroll(0, containerHeight());

    onCleanup(() => {
      observer.disconnect();
      scrollContainer?.removeEventListener("scroll", handleScroll);
    });
  });

  return (
    <div 
      ref={scrollContainer} 
      class="grid-view-container"
      role="grid"
      aria-label="Image gallery - grid layout"
      tabIndex={0}
    >
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
                style={{
                  position: "absolute",
                  top: 0,
                  left: 0,
                  transform: `translate3d(${pos.x}px, ${pos.y}px, 0)`,
                  width: `${pos.width}px`,
                  height: `${pos.height}px`,
                }}
                // Callbacks
                onSelect={actions.handleSelect}
                onOpen={actions.handleOpen}
                // DnD - drag source only (drop handled at container level)
                getSelectedIds={actions.getSelectedIds}
                getItemInfo={getItemInfo}
              />
            );
          }}
        </For>
      </div>
    </div>
  );
};
