import { Component } from "solid-js";
import { Table, Column } from "../../ui/Table";
import { useLibrary, useSelection, useViewport, useFilters } from "../../../core/hooks";
import { ImageItem } from "../../../types";
import { formatFileSize, formatDate } from "../../../utils/format";

export const VirtualListView: Component = () => {
    const lib = useLibrary();
    const selection = useSelection();
    const viewport = useViewport();
    const filters = useFilters();

    const getThumbUrl = (path: string | null) => {
        if (!path) return undefined;
        const filename = path.split(/[\\/]/).pop();
        return `thumb://localhost/${filename}`;
    };

    const columns: Column<ImageItem>[] = [
        {
            header: "",
            accessorKey: "thumbnail_path",
            width: 50,
            align: "center",
            cell: (item) => (
                <>
                    {item.thumbnail_path ? (
                        <img 
                            src={getThumbUrl(item.thumbnail_path)} 
                            alt="" 
                            style={{ "max-width": "32px", "max-height": "24px", "object-fit": "cover" }}
                        />
                    ) : (
                        <div style={{ width: "32px", height: "24px", background: "var(--surface-hover)", "border-radius": "2px" }} />
                    )}
                </>
            )
        },
        {
            header: "Name",
            accessorKey: "filename",
            sortable: true,
            width: "30%"
        },
        {
            header: "Type",
            accessorKey: "format",
            sortable: true,
            width: 80,
            align: "center",
            cell: (item) => <span class="uppercase opacity-70">{item.filename.split('.').pop()?.toUpperCase() || "N/A"}</span>
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
            cell: (item) => <span>{formatDate(item.created_at)}</span>
        },
        {
            header: "Modified",
            accessorKey: "modified_at",
            sortable: true,
            width: 160,
            cell: (item) => <span>{formatDate(item.modified_at)}</span>
        },
        {
            header: "Added",
            accessorKey: "added_at",
            sortable: true,
            width: 160,
            cell: (item) => <span>{formatDate(item.added_at)}</span>
        }
    ];

    const handleSort = (key: string, order: "asc" | "desc" | null) => {
        // Map table sort fields to our store sort fields if needed
        const fieldMap: Record<string, any> = {
            filename: "title",
            format: "type",
            size: "size",
            created_at: "creation",
            modified_at: "modification",
            added_at: "addition"
        };
        
        if (order) {
            filters.setSortBy(fieldMap[key] || key);
            filters.setSortOrder(order);
        }
    };

    const handleScroll = (e: Event) => {
        const target = e.currentTarget as HTMLDivElement;
        if (target.scrollTop + target.clientHeight >= target.scrollHeight - 500) {
            lib.loadMore();
        }
    };

    return (
        <div class="virtual-list-view">
            <Table
                data={lib.items}
                columns={columns}
                height="100%"
                selectedIds={selection.selectedIds}
                sortKey={Object.keys({
                    title: "filename",
                    type: "format",
                    size: "size",
                    creation: "created_at",
                    modification: "modified_at",
                    addition: "added_at"
                }).find(key => key === filters.sortBy) || filters.sortBy}
                sortOrder={filters.sortOrder as "asc" | "desc"}
                onSort={handleSort}
                onScroll={handleScroll}
                onRowClick={(item, multi) => selection.toggle(item.id, multi)}
                onRowDoubleClick={(item) => viewport.openItem(item.id.toString())}
                keyField="id"
            />
        </div>
    );
};
