import { Component, createMemo, Show } from "solid-js";
import { Table, Column } from "../../ui/Table";
import { useLibrary, useSelection, useViewport, useFilters } from "../../../core/hooks";
import { ImageItem } from "../../../types";
import { formatFileSize, formatDate } from "../../../utils/format";
import { assetDnD } from "../../../core/dnd";
import { ImageOff } from "lucide-solid";
import { EmptyState } from "./EmptyState";
import { createConditionalScope } from "../../../core/input";

export const VirtualListView: Component = () => {
    const lib = useLibrary();
    const selection = useSelection();
    const viewport = useViewport();
    const filters = useFilters();

    // Register viewport scope
    createConditionalScope('viewport', () => lib.items.length > 0);

    const getThumbUrl = (path: string | null) => {
        if (!path) return undefined;
        // Don't just take the filename! 'extensions/icon_xxx.webp' needs the full path.
        const normalizedPath = path.replace(/\\/g, '/');
        return `thumb://localhost/${normalizedPath}`;
    };

    const listThumbWidth = createMemo(() => Math.floor(filters.thumbSize / 5));
    const listThumbHeight = createMemo(() => Math.floor(listThumbWidth() * 0.75));
    const rowHeight = createMemo(() => Math.max(32, listThumbHeight() + 8));

    const columns = createMemo<Column<ImageItem>[]>(() => [
        {
            header: "",
            accessorKey: "thumbnail_path",
            width: listThumbWidth() + 16,
            align: "center",
            cell: (item) => (
                <div 
                    class="list-view-thumbnail-container"
                    style={{ 
                        width: `${listThumbWidth()}px`, 
                        height: `${listThumbHeight()}px`
                    }}
                >
                    {item.thumbnail_path && (
                        <img 
                            src={getThumbUrl(item.thumbnail_path)} 
                            alt="" 
                            draggable={false}
                            class="list-view-thumbnail"
                        />
                    )}
                </div>
            )
        },
        {
            header: "Name",
            accessorKey: "filename",
            sortable: true,
            width: 300
        },
        {
            header: "Rating",
            accessorKey: "rating",
            sortable: true,
            width: 100,
            align: "center",
            cell: (item) => <span class="list-view-rating-cell">{item.rating ? "★".repeat(item.rating) : "-"}</span>
        },
        {
            header: "Type",
            accessorKey: "format",
            sortable: true,
            width: 80,
            align: "center",
            cell: (item) => <span class="list-view-type-cell">{item.format?.toUpperCase() || "N/A"}</span>
        },
        {
            header: "Size",
            accessorKey: "size",
            sortable: true,
            width: 100,
            align: "right",
            cell: (item) => <span>{formatFileSize(item.size)}</span>
        },
        {
            header: "Dimensions",
            accessorKey: "width",
            width: 120,
            align: "center",
            cell: (item) => (
                <span>
                    {item.width && item.height ? `${item.width} × ${item.height}` : "-"}
                </span>
            )
        },
        {
            header: "Created",
            accessorKey: "created_at",
            sortable: true,
            width: 160,
            cell: (item) => <span class="list-view-date-cell">{formatDate(item.created_at)}</span>
        },
        {
            header: "Modified",
            accessorKey: "modified_at",
            sortable: true,
            width: 160,
            cell: (item) => <span class="list-view-date-cell">{formatDate(item.modified_at)}</span>
        },
        {
            header: "Added",
            accessorKey: "added_at",
            sortable: true,
            width: 160,
            cell: (item) => <span class="list-view-date-cell">{formatDate(item.added_at)}</span>
        }
    ]);

    const handleSort = (key: string) => {
        if (filters.sortBy === key) {
            // Cycle: asc -> desc -> asc
            const nextOrder = filters.sortOrder === "asc" ? "desc" : "asc";
            filters.setSortOrder(nextOrder);
        } else {
            filters.setSortBy(key as any);
            filters.setSortOrder("desc"); // Default to desc for new columns (usually more useful for dates/size)
        }
    };

    const handleScroll = (e: Event) => {
        const target = e.currentTarget as HTMLDivElement;
        if (target.scrollTop + target.clientHeight >= target.scrollHeight - 500) {
            lib.loadMore();
        }
    };

    // Navigation logic for Table is currently internal or via Table props. 
    // Table component needs to expose a way to be driven by external shortcuts OR use shortcuts internally.
    // Assuming Table handles its own focus/navigation, we just need to ensure it respects scopes?
    // If Table uses native onKeyDown, it won't respect our 'image-viewer' scope which blocks.
    // So Table MUST be refactored to use useShortcuts "viewport" scope.

    return (
        <div class="virtual-list-view">
            <Show when={lib.items.length > 0} fallback={
                <EmptyState 
                    title="No images found"
                    description="Try adjusting your filters or add images to your library."
                />
            }>
                <Table
                    data={lib.items}
                    columns={columns()}
                    height="100%"
                    rowHeight={rowHeight()}
                    selectedIds={selection.selectedIds}
                    sortKey={filters.sortBy}
                    sortOrder={filters.sortOrder as "asc" | "desc"}
                    onSort={handleSort}
                    onScroll={handleScroll}
                    onRowClick={(item, multi) => selection.toggle(item.id, multi)}
                    onRowDoubleClick={(item) => viewport.openItem(item.id.toString())}
                    onRowMount={(el, item) => assetDnD(el, () => ({ 
                        item, 
                        selected: selection.selectedIds.includes(item.id),
                        selectedIds: selection.selectedIds,
                        allItems: lib.items
                    }))}
                    keyField="id"
                    label="Image library list view"
                    emptyMessage="No images found"
                    emptyDescription="Try adjusting your filters or add images to your library."
                    emptyIcon={ImageOff}
                />
            </Show>
        </div>
    );
};
