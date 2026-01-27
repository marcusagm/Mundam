import { createSignal, onMount, onCleanup, For, createMemo } from "solid-js";
import { AssetCard } from "./AssetCard";
import { calculateMasonryLayout, type ImageItem } from "../../../utils/masonryLayout";
import { appActions, useAppStore } from "../../../core/store/appStore";
import "./viewport.css";

interface VirtualMasonryProps {
  items: ImageItem[];
}

export function VirtualMasonry(props: VirtualMasonryProps) {
  const { state } = useAppStore();
  let scrollContainer: HTMLDivElement | undefined;
  
  const [containerWidth, setContainerWidth] = createSignal(0);
  const [scrollTop, setScrollTop] = createSignal(0);
  const [containerHeight, setContainerHeight] = createSignal(0);
  
  // Configuration
  const minColWidth = 280;
  const gap = 16;
  
  // 1. Initial columns calculation based on width
  const columns = createMemo(() => {
    const width = containerWidth();
    if (width <= 0) return 4; // Default
    return Math.max(1, Math.floor((width + gap) / (minColWidth + gap)));
  });

  // 2. Synchronous layout calculation
  // Using createMemo ensures layout is always in sync with width/items
  const layout = createMemo(() => {
    return calculateMasonryLayout({
      items: props.items,
      columns: columns(),
      containerWidth: containerWidth(),
      gap
    });
  });

  // 3. Handle resize immediately (no debounce to prevent sync issues)
  onMount(() => {
    if (!scrollContainer) return;

    const observer = new ResizeObserver((entries) => {
      // Use requestAnimationFrame to avoid "ResizeObserver loop completed with undelivered notifications"
      requestAnimationFrame(() => {
          const entry = entries[0];
          if (!entry) return;
          
          const width = entry.contentRect.width;
          const height = entry.contentRect.height;
          
          setContainerHeight(height);
          if (width > 0 && Math.abs(width - containerWidth()) > 1) {
            setContainerWidth(width);
          }
      });
    });
    
    observer.observe(scrollContainer);
    
    // Initial measure
    const rect = scrollContainer.getBoundingClientRect();
    if (rect.width > 0) {
      setContainerWidth(rect.width);
      setContainerHeight(rect.height);
    } else {
        // Fallback: Estimate width if rect is 0 (e.g. hidden initially)
        // Sidebar is 80px fixed.
        const estimated = window.innerWidth - 80;
        if (estimated > 0) setContainerWidth(estimated);
    }
    
    const handleScroll = () => {
      if (!scrollContainer) return;
      setScrollTop(scrollContainer.scrollTop);
      
      const { scrollTop, scrollHeight, clientHeight } = scrollContainer;
      // Load more when user scrolls to within 500px of bottom
      if (scrollTop + clientHeight >= scrollHeight - 500) {
        appActions.loadMore();
      }
    };
    
    scrollContainer.addEventListener("scroll", handleScroll, { passive: true });
    
    onCleanup(() => {
      observer.disconnect();
      scrollContainer?.removeEventListener("scroll", handleScroll);
    });
  });
  
  // 4. Determine visible items
  // Optimized to only depend on layout and scroll
  const visibleItems = createMemo(() => {
    const sTop = scrollTop();
    const vHeight = containerHeight();
    const currentLayout = layout();
    const items = props.items;
    
    if (currentLayout.positions.size === 0 || items.length === 0) return [];

    const buffer = 1000; // Pixel buffer
    const rangeStart = sTop - buffer;
    const rangeEnd = sTop + vHeight + buffer;

    return items.filter(item => {
      const pos = currentLayout.positions.get(item.id);
      if (!pos) return false;
      return (pos.y + pos.height > rangeStart) && (pos.y < rangeEnd);
    });
  });

  return (
    <div 
      ref={scrollContainer} 
      class="virtual-scroll-container"
    >
      <div 
        class="virtual-track" 
        style={{ 
          height: `${layout().height}px`, 
        }}
      >
        <For each={visibleItems()}>
          {(item) => {
            // Access position derived from the SAME layout version as visibleItems
            // This prevents "tearing" where visibleItems updates but layout() hasn't
            const currentLayout = layout();
            const pos = currentLayout.positions.get(item.id);
            
            // Fallback for safety, though mapped correctly above
            if (!pos) return null;

            return (
              <AssetCard
                item={item}
                selected={state.selection.includes(item.id)}
                style={{
                  position: "absolute",
                  display: layout().positions.get(item.id) ? "block" : "none",
                  transform: `translate3d(${layout().positions.get(item.id)?.x ?? 0}px, ${layout().positions.get(item.id)?.y ?? 0}px, 0)`,
                  width: `${layout().positions.get(item.id)?.width ?? 0}px`,
                  height: `${layout().positions.get(item.id)?.height ?? 0}px`,
                  "margin-bottom": "0"
                }}
                onClick={(e) => {
                    e.stopPropagation();
                    appActions.toggleSelection(item.id, e.metaKey || e.ctrlKey);
                }}
              />
            );
          }}
        </For>
      </div>
    </div>
  );
}
