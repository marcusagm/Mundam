import { createSignal, onMount, onCleanup, For, createMemo, Show } from "solid-js";
import { AssetCard } from "./AssetCard";
import { EmptyState } from "./EmptyState";
import { type ImageItem } from "../../../types";
import {
  useLibrary,
  useAssetCardActions,
  useVirtualViewport,
  toLayoutItems,
} from "../../../core/hooks";
import "./viewport.css";

interface VirtualMasonryProps {
  items: ImageItem[];
  mode?: "masonry-v" | "masonry-h";
}

/**
 * VirtualMasonry - Worker-based Virtualized Masonry Layout
 *
 * Uses a Web Worker for layout calculations and Spatial Grid for O(1) visibility queries.
 * The main thread only renders items that are currently visible in the viewport.
 * 
 * Supports two modes:
 * - masonry-v: Vertical masonry (Pinterest-style) - fixed column width, variable height
 * - masonry-h: Horizontal masonry (Flickr-style) - fixed row height, variable width
 */
export function VirtualMasonry(props: VirtualMasonryProps) {
  const lib = useLibrary();
  const actions = useAssetCardActions();

  let scrollContainer: HTMLDivElement | undefined;

  const [_containerWidth, setContainerWidth] = createSignal(0);
  const [containerHeight, setContainerHeight] = createSignal(0);

  // Convert items to Worker-friendly format (minimal data)
  const layoutItems = createMemo(() => toLayoutItems(props.items));

  // Connect to the layout Worker with the specified mode (as accessor for reactivity)
  const layoutMode = () => props.mode || "masonry-v";
  const viewport = useVirtualViewport(layoutMode, layoutItems);

  // Create item lookup Map for O(1) access during render
  const itemsById = createMemo(() => {
    const map = new Map<number, ImageItem>();
    props.items.forEach((item) => map.set(item.id, item));
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

  // Handle resize and scroll
  // Track last width sent to Worker to prevent loops
  let lastReportedWidth = 0;
  
  onMount(() => {
    if (!scrollContainer) return;

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

    observer.observe(scrollContainer);

    // Initial measure
    const rect = scrollContainer.getBoundingClientRect();
    if (rect.width > 0) {
      setContainerWidth(rect.width);
      setContainerHeight(rect.height);
      lastReportedWidth = rect.width;
      viewport.handleResize(rect.width);
    } else {
      const estimated = window.innerWidth - 80;
      if (estimated > 0) {
        setContainerWidth(estimated);
        lastReportedWidth = estimated;
        viewport.handleResize(estimated);
      }
    }

    const handleScroll = () => {
      if (!scrollContainer) return;
      const currentScrollTop = scrollContainer.scrollTop;

      // Notify Worker of scroll position
      viewport.handleScroll(currentScrollTop, containerHeight());

      // Load more when near bottom
      const { scrollTop: sTop, scrollHeight, clientHeight } = scrollContainer;
      if (sTop + clientHeight >= scrollHeight - 500) {
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
      class="virtual-scroll-container"
      role="grid"
      aria-label="Image gallery - masonry layout"
      tabIndex={0}
    >
      <Show when={props.items.length > 0} fallback={
        <EmptyState 
          title="No images found"
          description="Try adjusting your filters or add images to your library."
        />
      }>
      <div
        class="virtual-track"
        role="rowgroup"
        style={{
          height: `${viewport.totalHeight()}px`,
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
      </Show>
    </div>
  );
}
