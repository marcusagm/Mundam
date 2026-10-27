import { Component, createMemo, Show, createSignal, JSX } from 'solid-js';
import { Table, Column } from '../../../ui/Table';
import {
    useLibrary,
    useSelection,
    useViewport,
    useFilters,
    useAssetCardActions
} from '../../../../core/hooks';
import { type SortField } from '../../../../core/store/filter';
import { AssetItem } from '../../../../types';
import { formatFileSize, formatDate } from '../../../../utils/format';
import { assetDnD } from '../../../../core/dnd';
import { EmptyState } from '../components/EmptyState';
import { AssetContextMenu } from '../components/AssetContextMenu';
import { createConditionalScope } from '../../../../core/input';
import { scheduler } from '../../../../core/utils/scheduler';
import {
    listColumnConfigs,
    updateListColumnConfig,
    isListColumnHidden,
    type ListColumnConfig
} from '../../../../core/store/listColumnsStore';
import { FileIcon } from '../assets/FileIcon';
import { getAssetUrl } from '../../../../core/utils/assetUrl';

/**
 * Thumbnail scale factor
 */
const THUMBNAIL_SCALE_FACTOR = 5;

/**
 * Thumbnail aspect ratio
 */
const THUMBNAIL_ASPECT_RATIO = 0.75;

/**
 * Minimum row height
 */
const LIST_MIN_ROW_HEIGHT = 32;

/**
 * Row padding
 */
const LIST_ROW_PADDING = 8;

/**
 * Scroll load more threshold
 */
const SCROLL_LOAD_MORE_THRESHOLD = 500;

/**
 * Default column widths
 */
const DEFAULT_COLS = {
    thumbnailPadding: 16,
    filename: 300,
    rating: 100,
    format: 80,
    size: 100,
    width: 120,
    date: 160
} as const;

/**
 * Extracts the lowercase extension (with dot) from a filename.
 */
const getFileExtension = (filename: string): string => {
    const lastDot = filename.lastIndexOf('.');
    return lastDot === -1 ? '' : filename.substring(lastDot).toLowerCase();
};

/**
 * Virtual list view component
 *
 * @returns {JSX.Element} The virtual list view.
 */
export const VirtualListView: Component = (): JSX.Element => {
    let lastPriorityKey = '';
    /**
     * Library store
     *
     * @returns {Library} The library store.
     */
    const lib = useLibrary();

    /**
     * Selection store
     *
     * @returns {Selection} The selection store.
     */
    const selection = useSelection();

    /**
     * Viewport store
     *
     * @returns {Viewport} The viewport store.
     */
    const viewport = useViewport();

    /**
     * Filters store
     *
     * @returns {Filters} The filters store.
     */
    const filters = useFilters();

    /**
     * Asset card actions store
     *
     * @returns {AssetCardActions} The asset card actions store.
     */
    const actions = useAssetCardActions();

    /**
     * Register viewport scope
     *
     * @returns {void}
     */
    createConditionalScope('viewport', () => lib.items.length > 0);

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
     * @param {AssetItem} item - Asset item
     */
    const handleContextMenu = (event: MouseEvent, item: AssetItem) => {
        event.preventDefault();
        setContextMenuAsset(item);
        setContextMenuPosition({ coordinateX: event.clientX, coordinateY: event.clientY });
        setContextMenuOpen(true);
    };

    /**
     * Get thumbnail URL
     *
     * @param {string | null} path - The path to the thumbnail.
     * @returns {string | undefined} The thumbnail URL.
     */
    const getThumbUrl = (id: string, path: string | null) => {
        if (!path) return undefined;
        return getAssetUrl(id, 'thumb');
    };

    /**
     * Update column configuration
     *
     * @param {string} key - The column key.
     * @param {ListColumnConfig} updates - The column updates.
     * @returns {void}
     */
    const updateColumnConfig = (key: string, updates: ListColumnConfig) =>
        updateListColumnConfig(key, updates);

    /**
     * Get column width
     *
     * @param {string} key - The column key.
     * @param {number} defaultWidth - The default column width.
     * @returns {number} The column width.
     */
    const getColumnWidth = (key: string, defaultWidth: number) => {
        return listColumnConfigs()[key]?.width ?? defaultWidth;
    };

    /**
     * Check if column is hidden
     *
     * @param {string} key - The column key.
     * @param {boolean} defaultHidden - The default column hidden state.
     * @returns {boolean} The column hidden state.
     */
    const isColumnHidden = (key: string, defaultHidden: boolean) =>
        isListColumnHidden(key, defaultHidden);

    /**
     * List thumbnail width
     */
    const listThumbWidth = createMemo(() => Math.floor(filters.thumbSize / THUMBNAIL_SCALE_FACTOR));

    /**
     * List thumbnail height
     */
    const listThumbHeight = createMemo(() => Math.floor(listThumbWidth() * THUMBNAIL_ASPECT_RATIO));

    /**
     * Row height
     */
    const rowHeight = createMemo(() =>
        Math.max(LIST_MIN_ROW_HEIGHT, listThumbHeight() + LIST_ROW_PADDING)
    );

    /**
     * Columns
     */
    const columns = createMemo<Column<AssetItem>[]>(() => [
        {
            header: '',
            accessorKey: 'thumbnail_path',
            width: getColumnWidth(
                'thumbnail_path',
                listThumbWidth() + DEFAULT_COLS.thumbnailPadding
            ),
            align: 'center',
            resizable: true,
            toggleable: false,
            cell: (item: AssetItem) => (
                <div
                    class="list-view-thumbnail-container"
                    style={{
                        width: `${listThumbWidth()}px`,
                        height: `${listThumbHeight()}px`
                    }}
                >
                    <Show
                        when={item.thumbnail_path}
                        fallback={
                            <FileIcon
                                extension={getFileExtension(item.filename)}
                                mediaType={item.media_type || 'Unknown'}
                            />
                        }
                    >
                        <img
                            src={getThumbUrl(item.id, item.thumbnail_path)}
                            alt=""
                            draggable={false}
                            class="list-view-thumbnail"
                        />
                    </Show>
                </div>
            )
        },
        {
            header: 'Name',
            accessorKey: 'filename',
            sortable: true,
            resizable: true,
            toggleable: false,
            width: getColumnWidth('filename', DEFAULT_COLS.filename)
        },
        {
            header: 'Rating',
            accessorKey: 'rating',
            sortable: true,
            resizable: true,
            width: getColumnWidth('rating', DEFAULT_COLS.rating),
            hidden: isColumnHidden('rating', false),
            align: 'center',
            cell: (item: AssetItem) => (
                <span class="list-view-rating-cell">
                    {item.rating ? '★'.repeat(item.rating) : '-'}
                </span>
            )
        },
        {
            header: 'Type',
            accessorKey: 'format',
            sortable: true,
            resizable: true,
            width: getColumnWidth('format', DEFAULT_COLS.format),
            hidden: isColumnHidden('format', false),
            align: 'center',
            cell: (item: AssetItem) => (
                <span class="list-view-type-cell">{item.format?.toUpperCase() || 'N/A'}</span>
            )
        },
        {
            header: 'Size',
            accessorKey: 'size',
            sortable: true,
            resizable: true,
            width: getColumnWidth('size', DEFAULT_COLS.size),
            hidden: isColumnHidden('size', false),
            align: 'right',
            cell: (item: AssetItem) => <span>{formatFileSize(item.size)}</span>
        },
        {
            header: 'Dimensions',
            accessorKey: 'width',
            resizable: true,
            width: getColumnWidth('width', DEFAULT_COLS.width),
            hidden: isColumnHidden('width', false),
            align: 'center',
            cell: (item: AssetItem) => (
                <span>{item.width && item.height ? `${item.width} × ${item.height}` : '-'}</span>
            )
        },
        {
            header: 'Created',
            accessorKey: 'created_at',
            sortable: true,
            resizable: true,
            width: getColumnWidth('created_at', DEFAULT_COLS.date),
            hidden: isColumnHidden('created_at', true),
            cell: (item: AssetItem) => (
                <span class="list-view-date-cell">{formatDate(item.created_at)}</span>
            )
        },
        {
            header: 'Modified',
            accessorKey: 'modified_at',
            sortable: true,
            resizable: true,
            width: getColumnWidth('modified_at', DEFAULT_COLS.date),
            hidden: isColumnHidden('modified_at', true),
            cell: (item: AssetItem) => (
                <span class="list-view-date-cell">{formatDate(item.modified_at)}</span>
            )
        },
        {
            header: 'Added',
            accessorKey: 'added_at',
            sortable: true,
            resizable: true,
            width: getColumnWidth('added_at', DEFAULT_COLS.date),
            hidden: isColumnHidden('added_at', true),
            cell: (item: AssetItem) => (
                <span class="list-view-date-cell">{formatDate(item.added_at)}</span>
            )
        }
    ]);

    /**
     * Handle sort
     */
    const handleSort = (key: string) => {
        if (filters.sortBy === key) {
            const nextOrder = filters.sortOrder === 'asc' ? 'desc' : 'asc';
            filters.setSortOrder(nextOrder);
        } else {
            filters.setSortBy(key as SortField);
            filters.setSortOrder('desc');
        }
    };

    /**
     * Handle scroll
     */
    let isScrollScheduled = false;

    /**
     * Handle scroll
     */
    const handleScroll = (event: Event) => {
        const target = event.currentTarget as HTMLDivElement;

        if (!isScrollScheduled) {
            isScrollScheduled = true;
            scheduler.schedule(() => {
                if (
                    target.scrollTop + target.clientHeight >=
                    target.scrollHeight - SCROLL_LOAD_MORE_THRESHOLD
                ) {
                    lib.loadMore();
                }
                isScrollScheduled = false;
            });
        }
    };

    return (
        <div class="virtual-list-view">
            <Show when={lib.items.length > 0} fallback={<EmptyState />}>
                <Table
                    data={lib.items}
                    columns={columns()}
                    rowHeight={rowHeight()}
                    height="100%"
                    sortKey={filters.sortBy}
                    sortOrder={filters.sortOrder}
                    selectedIds={selection.selectedIds}
                    onSort={handleSort}
                    onColumnResize={(key: string, width: number) =>
                        updateColumnConfig(key, { width })
                    }
                    onColumnVisibilityChange={(key: string, visible: boolean) =>
                        updateColumnConfig(key, { hidden: !visible })
                    }
                    onRowClick={(item: AssetItem, multi: boolean, shift: boolean) => {
                        actions.handleSelect(item.id, { multi, shift });
                    }}
                    onRowDoubleClick={(item: AssetItem) => {
                        viewport.openItem(item.id.toString());
                    }}
                    onRowContextMenu={handleContextMenu}
                    onScroll={handleScroll}
                    onRowMount={(element: HTMLElement, item: AssetItem) => {
                        assetDnD(element, () => ({
                            item,
                            selected: selection.isSelected(item.id),
                            selectedIds: selection.selectedIds,
                            allItems: lib.items
                        }));
                    }}
                    onVisibleItemsChange={(items: AssetItem[]) => {
                        const idsToPrioritize = items
                            .filter(item => !item.thumbnail_path)
                            .map((item: AssetItem) => item.id);

                        // Skip repeated requests for the same set (e.g. triggered by data updates)
                        const requestKey = idsToPrioritize.join(',');
                        if (requestKey === lastPriorityKey) return;
                        lastPriorityKey = requestKey;

                        if (idsToPrioritize.length > 0) {
                            lib.setThumbnailPriority(idsToPrioritize);
                        }
                    }}
                />
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
