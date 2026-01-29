import { Component, createSignal, onMount, onCleanup, For, createMemo } from "solid-js";
import { AssetCard } from "./AssetCard";
import { useLibrary, useSelection, useFilters } from "../../../core/hooks";
import "./grid-view.css";

export const VirtualGridView: Component = () => {
    const lib = useLibrary();
    const selection = useSelection();
    const filters = useFilters();
    
    let scrollContainer: HTMLDivElement | undefined;
    
    const [containerWidth, setContainerWidth] = createSignal(0);
    const [scrollTop, setScrollTop] = createSignal(0);
    const [containerHeight, setContainerHeight] = createSignal(0);

    const itemSize = createMemo(() => filters.thumbSize);
    const gap = 16;
    
    const cols = createMemo(() => {
        const width = containerWidth();
        if (width <= 0) return 4;
        return Math.max(1, Math.floor((width + gap) / (itemSize() + gap)));
    });

    onMount(() => {
        if (!scrollContainer) return;
        
        const observer = new ResizeObserver((entries) => {
            const entry = entries[0];
            if (entry) {
                setContainerWidth(entry.contentRect.width);
                setContainerHeight(entry.contentRect.height);
            }
        });
        
        observer.observe(scrollContainer);
        setContainerWidth(scrollContainer.clientWidth);
        setContainerHeight(scrollContainer.clientHeight);
        
        const handleScroll = () => {
            if (!scrollContainer) return;
            setScrollTop(scrollContainer.scrollTop);
            
            if (scrollContainer.scrollTop + scrollContainer.clientHeight >= scrollContainer.scrollHeight - 500) {
                lib.loadMore();
            }
        };
        
        scrollContainer.addEventListener("scroll", handleScroll, { passive: true });
        
        onCleanup(() => {
            observer.disconnect();
            scrollContainer?.removeEventListener("scroll", handleScroll);
        });
    });

    const totalRows = createMemo(() => Math.ceil(lib.items.length / cols()));
    const totalHeight = createMemo(() => totalRows() * (itemSize() + gap));

    const visibleRange = createMemo(() => {
        const sTop = scrollTop();
        const cHeight = containerHeight();
        const rowH = itemSize() + gap;
        
        const startRow = Math.max(0, Math.floor(sTop / rowH) - 4);
        const visibleRows = Math.ceil(cHeight / rowH) + 8;
        const endRow = Math.min(totalRows(), startRow + visibleRows);
        
        return {
            start: startRow * cols(),
            end: endRow * cols(),
            startRow
        };
    }, undefined, { 
        equals: (a, b) => a.start === b.start && a.end === b.end 
    });

    return (
        <div ref={scrollContainer} class="grid-view-container">
            <div 
                class="grid-view-track" 
                style={{ 
                    height: `${totalHeight()}px`,
                    position: "relative"
                }}
            >
                <For each={lib.items.slice(visibleRange().start, visibleRange().end)}>
                    {(item, index) => {
                        const globalIndex = createMemo(() => visibleRange().start + index());
                        const row = createMemo(() => Math.floor(globalIndex() / cols()));
                        const col = createMemo(() => globalIndex() % cols());
                        const x = createMemo(() => col() * (itemSize() + gap));
                        const y = createMemo(() => row() * (itemSize() + gap));

                        return (
                            <AssetCard
                                item={item}
                                selected={selection.selectedIds.includes(item.id)}
                                style={{
                                    position: "absolute",
                                    top: 0,
                                    left: 0,
                                    transform: `translate3d(${x()}px, ${y()}px, 0)`,
                                    width: `${itemSize()}px`,
                                    height: `${itemSize()}px`
                                }}
                                onClick={(e) => {
                                    e.stopPropagation();
                                    selection.toggle(item.id, e.metaKey || e.ctrlKey);
                                }}
                            />
                        );
                    }}
                </For>
            </div>
        </div>
    );
};
