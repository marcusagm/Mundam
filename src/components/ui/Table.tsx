import { JSX, createSignal, createMemo, onMount, onCleanup, For, Show, splitProps, Component } from "solid-js";
import { ChevronUp, ChevronDown, ChevronsUpDown, Inbox } from "lucide-solid";
import type { LucideProps } from "lucide-solid";
import { cn } from "../../lib/utils";
import { shortcutStore } from "../../core/input/store/shortcutStore";
import "./table.css";

export interface Column<T> {
  /** The text or element to display in the header */
  header: string | JSX.Element;
  /** The key in the data object to access the value */
  accessorKey: keyof T | string;
  /** Optional width (e.g. '100px', '20%', or 150) */
  width?: string | number;
  /** Optional custom cell renderer */
  cell?: (item: T) => JSX.Element;
  /** Whether the column is sortable */
  sortable?: boolean;
  /** Whether the column is currently hidden */
  hidden?: boolean;
  /** Optional alignment */
  align?: "left" | "center" | "right";
  /** Is this a pinned column? (Future-proofing) */
  pinned?: "left" | "right";
}

export type SortOrder = "asc" | "desc" | null;

export interface TableProps<T> {
  /** Array of data items to display */
  data: T[];
  /** Column definitions */
  columns: Column<T>[];
  /** Fixed height of each row in pixels */
  rowHeight?: number;
  /** Whether the header should stick to the top when scrolling */
  stickyHeader?: boolean;
  /** Currently active sort key */
  sortKey?: string | null;
  /** Currently active sort order */
  sortOrder?: SortOrder;
  /** Selected item IDs */
  selectedIds?: (string | number)[];
  /** Callback when sort changes */
  onSort?: (key: string, order: SortOrder) => void;
  /** Callback when a row is clicked */
  onRowClick?: (item: T, multi: boolean, range: boolean) => void;
  /** Callback for double click */
  onRowDoubleClick?: (item: T) => void;
  /** Callback for scroll events */
  onScroll?: (e: Event) => void;
  /** Callback when a row element is mounted */
  onRowMount?: (el: HTMLElement, item: T) => void;
  /** Key field to use for identifying rows (default: 'id') */
  keyField?: keyof T;
  /** Additional CSS class for the container */
  class?: string;
  /** Height of the table container (required for virtualization) */
  height?: string | number;
  /** ARIA label for the grid */
  label?: string;
  /** Message to display when table is empty */
  emptyMessage?: string;
  /** Description for empty state */
  emptyDescription?: string;
  /** Custom icon for empty state (lucide-solid icon component) */
  emptyIcon?: Component<LucideProps>;
}

/**
 * Table Component - Perfected ARIA Grid Pattern
 * Optimized for high-performance DAM listing with virtualization and desktop-class UX.
 */
export function Table<T>(props: TableProps<T>) {
  const [local] = splitProps(props, [
    "data",
    "columns",
    "rowHeight",
    "stickyHeader",
    "sortKey",
    "sortOrder",
    "selectedIds",
    "onSort",
    "onRowClick",
    "onRowDoubleClick",
    "onScroll",
    "onRowMount",
    "keyField",
    "class",
    "height",
    "label",
    "emptyMessage",
    "emptyDescription",
    "emptyIcon"
  ]);

  let gridContainer: HTMLDivElement | undefined;
  
  const [scrollTop, setScrollTop] = createSignal(0);
  const [containerHeight, setContainerHeight] = createSignal(0);
  const [focusedIndex, setFocusedIndex] = createSignal(-1);

  const rowHeight = () => local.rowHeight ?? 32; // Standard compact height for DAM
  const HEADER_HEIGHT = 32; // Matches .ui-table-grid-header height in CSS
  const stickyHeader = () => local.stickyHeader ?? true;
  const keyField = () => local.keyField ?? ("id" as keyof T);
  const selectedIds = () => local.selectedIds ?? [];
  
  // Filter visible columns
  const visibleColumns = createMemo(() => 
    local.columns.filter(col => !col.hidden)
  );

  // Measure container height
  onMount(() => {
    if (!gridContainer) return;
    
    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (entry) {
        setContainerHeight(entry.contentRect.height);
      }
    });

    observer.observe(gridContainer);
    setContainerHeight(gridContainer.clientHeight);

    onCleanup(() => observer.disconnect());
  });

  const handleScroll = (e: Event) => {
    const target = e.currentTarget as HTMLDivElement;
    setScrollTop(target.scrollTop);
    local.onScroll?.(e);
  };

  const handleSort = (key: string) => {
    if (!local.onSort) return;
    
    let nextOrder: SortOrder = "asc";
    if (local.sortKey === key) {
      if (local.sortOrder === "asc") nextOrder = "desc";
      else if (local.sortOrder === "desc") nextOrder = null;
    }
    
    local.onSort(key, nextOrder);
  };

  // Keyboard Navigation Manager
  // Standardized with Keyboad Shortcuts Store
  const isCommand = (e: KeyboardEvent, command: string, defaultKey: string): boolean => {
    const shortcut = shortcutStore.getByCommand(command);
    if (!shortcut) return e.key === defaultKey;
    
    const keysArray = Array.isArray(shortcut.keys) ? shortcut.keys : [shortcut.keys];
    
    return keysArray.some((k: string) => 
      k === e.key || 
      k === e.code || 
      k.toLowerCase() === e.key.toLowerCase() ||
      k.replace('Key', '') === e.key.toUpperCase()
    );
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    const dataLen = local.data.length;
    if (dataLen === 0) return;

    let nextIndex = focusedIndex();
    let handled = false;

    // Ignore input fields unless explicitly allowed
    if (["INPUT", "TEXTAREA"].includes((e.target as HTMLElement).tagName)) {
        return;
    }

    if (isCommand(e, 'viewport:move-down', 'ArrowDown')) {
      handled = true;
      e.preventDefault();
      nextIndex = Math.min(dataLen - 1, nextIndex + 1);
    } 
    else if (isCommand(e, 'viewport:move-up', 'ArrowUp')) {
      handled = true;
      e.preventDefault();
      nextIndex = Math.max(0, nextIndex - 1);
    }
    else if (isCommand(e, 'viewport:home', 'Home')) {
      handled = true;
      e.preventDefault();
      nextIndex = 0;
    }
    else if (isCommand(e, 'viewport:end', 'End')) {
      handled = true;
      e.preventDefault();
      nextIndex = dataLen - 1;
    }
    else if (isCommand(e, 'viewport:open', 'Enter')) {
      // Enter opens the item
      handled = true;
      e.preventDefault();
      if (nextIndex >= 0) {
        const item = local.data[nextIndex];
        local.onRowDoubleClick?.(item);
      }
    }
    else if (isCommand(e, 'viewport:toggle-select', ' ')) {
      // Space toggles selection
      handled = true;
      e.preventDefault();
      if (nextIndex >= 0) {
        const item = local.data[nextIndex];
        // Shift+Space = add to selection, Space alone = toggle
        local.onRowClick?.(item, e.shiftKey, false);
      }
    }

    if (handled && nextIndex !== focusedIndex()) {
      setFocusedIndex(nextIndex);
      // Ensure the focused row is visible (scroll management)
      const rHeight = rowHeight();
      const sTop = scrollTop();
      const cHeight = containerHeight();
      const hHeight = HEADER_HEIGHT; // Actual header height

      const itemTop = nextIndex * rHeight;
      const itemBottom = itemTop + rHeight;

      if (itemTop < sTop + hHeight) {
        gridContainer?.scrollTo({ top: itemTop - hHeight });
      } else if (itemBottom > sTop + cHeight) {
        gridContainer?.scrollTo({ top: itemBottom - cHeight + hHeight });
      }
    }
  };

  // Virtualization logic
  const visibleRange = createMemo(() => {
    const sTop = scrollTop();
    const cHeight = containerHeight();
    const rHeight = rowHeight();
    
    const start = Math.max(0, Math.floor((sTop - HEADER_HEIGHT) / rHeight) - 5);
    const visibleCount = Math.ceil(cHeight / rHeight);
    const end = Math.min(local.data.length, start + visibleCount + 10);
    
    return { start, end };
  });

  const totalHeight = createMemo(() => local.data.length * rowHeight());

  return (
    <div 
      ref={gridContainer}
      class={cn("ui-table-grid-container", local.class)}
      style={{ height: typeof local.height === 'number' ? `${local.height}px` : local.height }}
      onScroll={handleScroll}
      onKeyDown={handleKeyDown}
      role="grid"
      aria-label={local.label || "Data Table"}
      aria-rowcount={local.data.length}
      aria-colcount={visibleColumns().length}
      aria-multiselectable="true"
      tabindex="0"
    >
      <div 
        class="ui-table-grid-track"
        style={{ height: `${totalHeight() + HEADER_HEIGHT}px` }}
        role="presentation"
      >
        {/* Header Row */}
        <div 
          class={cn(
            "ui-table-grid-header", 
            stickyHeader() && "ui-table-grid-header-sticky"
          )}
          role="row"
          aria-rowindex="1"
        >
          <For each={visibleColumns()}>
            {(col) => (
              <div 
                class={cn(
                  "ui-table-grid-th",
                  col.sortable && "ui-table-grid-th-sortable",
                  local.sortKey === col.accessorKey && "ui-table-grid-th-active"
                )}
                style={{ 
                  width: typeof col.width === 'number' ? `${col.width}px` : col.width || '150px', 
                  flex: col.width ? '0 0 auto' : '1 1 0',
                  "justify-content": col.align === 'center' ? 'center' : col.align === 'right' ? 'flex-end' : 'flex-start'
                }}
                onClick={() => col.sortable && handleSort(col.accessorKey as string)}
                role="columnheader"
                aria-sort={local.sortKey === col.accessorKey ? (local.sortOrder === "asc" ? "ascending" : "descending") : "none"}
              >
                <span class="ui-table-grid-th-text">{col.header}</span>
                <Show when={col.sortable}>
                  <span class="ui-table-grid-sort-icon">
                    {local.sortKey === col.accessorKey ? (
                      local.sortOrder === "asc" ? <ChevronUp size={12} /> : <ChevronDown size={12} />
                    ) : (
                      <ChevronsUpDown size={12} />
                    )}
                  </span>
                </Show>
              </div>
            )}
          </For>
        </div>

        {/* Dynamic Rows or Empty State */}
        <Show 
          when={local.data.length > 0} 
          fallback={(() => {
            const Icon = local.emptyIcon || Inbox;
            return (
              <div class="ui-table-empty-state" role="row" aria-rowindex="2">
                <div class="ui-table-empty-icon">
                  <Icon size={48} />
                </div>
                <div class="ui-table-empty-message">
                  {local.emptyMessage || "No items to display"}
                </div>
                <div class="ui-table-empty-description">
                  {local.emptyDescription || "Adjust your filters or add items to see data here."}
                </div>
              </div>
            );
          })()}
        >
          <For each={local.data.slice(visibleRange().start, visibleRange().end)}>
            {(item, index) => {
              const realIndex = createMemo(() => visibleRange().start + index());
              const id = (item[keyField()] as any);
              const isSelected = createMemo(() => selectedIds().includes(id));
              const isFocused = createMemo(() => focusedIndex() === realIndex());

              return (
                <div 
                  ref={(el) => local.onRowMount?.(el, item)}
                  class={cn(
                    "ui-table-grid-row",
                    isSelected() && "ui-table-grid-row-selected",
                    isFocused() && "ui-table-grid-row-focused"
                  )}
                  style={{ 
                    height: `${rowHeight()}px`, 
                    transform: `translate3d(0, ${HEADER_HEIGHT + realIndex() * rowHeight()}px, 0)` 
                  }}
                  onClick={(e) => {
                    setFocusedIndex(realIndex());
                    local.onRowClick?.(item, e.ctrlKey || e.metaKey, e.shiftKey);
                    gridContainer?.focus({ preventScroll: true });
                  }}
                  onDblClick={() => local.onRowDoubleClick?.(item)}
                  role="row"
                  aria-rowindex={realIndex() + 2} // +1 for index and +1 for header
                  aria-selected={isSelected()}
                >
                  <For each={visibleColumns()}>
                    {(col) => (
                      <div 
                        class="ui-table-grid-cell"
                        style={{ 
                          width: typeof col.width === 'number' ? `${col.width}px` : col.width || '150px', 
                          flex: col.width ? '0 0 auto' : '1 1 0',
                          "justify-content": col.align === 'center' ? 'center' : col.align === 'right' ? 'flex-end' : 'flex-start'
                        }}
                        role="gridcell"
                        aria-colindex={visibleColumns().indexOf(col) + 1}
                      >
                        <div class="ui-table-grid-cell-content">
                          {col.cell ? col.cell(item) : (item[col.accessorKey as keyof T] as any)}
                        </div>
                      </div>
                    )}
                  </For>
                </div>
              );
            }}
          </For>
        </Show>
      </div>
    </div>
  );
}
