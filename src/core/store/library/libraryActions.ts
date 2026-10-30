import { reconcile } from 'solid-js/store';
import { untrack } from 'solid-js';
import { invokeCommand as invoke } from '../../../lib/api';

import { tagService } from '../../../lib/tags';
import { ActionResult, ErrorCode } from '../../types/actions';
import { filterState, filterActions } from '../filter';
import { selectionState } from '../selectionStore';
import { libraryState, libraryStateInternal } from './libraryState';
import { BatchChangePayload } from './schemas';
import { APP_CONFIG } from '../../../config/constants';

const { setLibraryState } = libraryStateInternal;
const BATCH_SIZE = APP_CONFIG.BATCH_SIZE;
let currentOffset = 0;
// Identifies the latest refresh so slower, older responses never overwrite newer data.
let latestRefreshRequestId = 0;

import { itemActions, setRefreshAssetsHandler } from './itemActions';
import { metadataActions, metadataState } from '../metadata';
import { tauriService } from '../../tauri/services';
import { addLocation as dbAddLocation } from '../../../lib/db';
import { open } from '@tauri-apps/plugin-dialog';
import { eventBus } from '../../utils/eventBus';
import type { SearchGroup, SearchCriterion } from '../../../types';
import type { SearchGroup as UISearchGroup } from '../filter/schemas';

const mapToSearchGroup = (group: UISearchGroup): SearchGroup => ({
    id: group.id,
    logicalOperator: group.logicalOperator,
    items: group.items.map(item => {
        if ('items' in item) {
            return mapToSearchGroup(item as UISearchGroup);
        }
        return {
            id: item.id,
            key: item.key,
            operator: item.operator,
            value: item.value
        } as SearchCriterion;
    })
});

let refreshDebounceTimer: ReturnType<typeof setTimeout> | null = null;

export const libraryActions = {
    ...itemActions,

    fetchLibraryBatch: async (offset: number) => {
        const anyFilter = filterActions.hasActiveFilters();

        let filterParams: import('../../../types').AssetFilter = {
            sortBy: filterState.sortBy,
            sortOrder: filterState.sortOrder
        };

        if (anyFilter) {
            filterParams = {
                ...filterParams,
                untagged: filterState.filterUntagged ? true : undefined,
                hasTags: filterState.filterHasTags ? true : undefined,
                favoritesOnly: filterState.filterFavorites ? true : undefined,
                trashOnly: filterState.filterTrash ? true : undefined,
                family: filterState.filterFamily || undefined,
                folderIds: filterState.selectedFolderIds.length
                    ? filterState.selectedFolderIds.map(String)
                    : undefined,
                recursive: filterState.folderRecursiveView,
                tags: filterState.selectedTags.map(String),
                searchQuery: filterState.searchQuery,
                searchFuzzy: filterState.searchFuzzy || undefined
            };

            if (filterState.advancedSearch) {
                return await tagService.searchAssets(
                    {
                        id: 'advanced-search',
                        rootGroup: mapToSearchGroup(filterState.advancedSearch),
                        sortBy: filterState.sortBy,
                        sortOrder: filterState.sortOrder
                    },
                    offset / BATCH_SIZE + 1, // converting offset to page
                    BATCH_SIZE
                );
            }
        }

        return await tagService.getAssets(filterParams, offset / BATCH_SIZE + 1, BATCH_SIZE);
    },

    refreshTotalCount: () => {
        // Assets and search endpoints already return totalItems.
        // Handled directly in refreshAssets and loadMore now.
    },

    refreshAssets: async (reset = false) => {
        if (libraryState.isRefreshing && reset) return;
        if (reset) setLibraryState('isRefreshing', true);

        const requestId = ++latestRefreshRequestId;

        try {
            const { items, total_items } = await libraryActions.fetchLibraryBatch(0);
            if (requestId !== latestRefreshRequestId) return;
            setLibraryState('items', reconcile(items, { key: 'id' }));
            setLibraryState('totalItems', total_items);
            currentOffset = BATCH_SIZE;

            libraryActions.refreshTotalCount();
        } finally {
            if (reset) setLibraryState('isRefreshing', false);
        }
    },

    loadMore: async () => {
        if (libraryState.isFetching) return;
        setLibraryState('isFetching', true);

        try {
            const { items, total_items } = await libraryActions.fetchLibraryBatch(currentOffset);

            if (items.length > 0) {
                setLibraryState('items', prev => [...prev, ...items]);
                setLibraryState('totalItems', total_items);
                currentOffset += BATCH_SIZE;
            }
        } finally {
            setLibraryState('isFetching', false);
        }
    },

    handleBatchChange: (payload: BatchChangePayload) => {
        if (payload.removed && payload.removed.length > 0) {
            const removedIds = new Set(payload.removed.map(removedItem => removedItem.id));
            setLibraryState('items', items => items.filter(item => !removedIds.has(item.id)));
        }

        if (payload.needs_refresh || (payload.added && payload.added.length > 0)) {
            // Debounce the refresh to prevent spam during mass file operations
            if (!refreshDebounceTimer) {
                refreshDebounceTimer = setTimeout(() => {
                    libraryActions.refreshAssets(false);
                    refreshDebounceTimer = null;
                }, 1500);
            }
        }

        if (payload.updated && payload.updated.length > 0) {
            const updatedItems = payload.updated;
            const { selectedFolderIds, recursive, locations, currentItems } = untrack(() => ({
                selectedFolderIds: [...filterState.selectedFolderIds],
                recursive: filterState.folderRecursiveView,
                locations: metadataState.locations,
                currentItems: libraryState.items
            }));

            const locationMap = new Map(locations.map(location => [location.id, location]));

            const isChildOf = (childId: string, rootId: string): boolean => {
                let current: string | null = childId;
                let depth = 0;
                while (current && depth < APP_CONFIG.MAX_FOLDER_DEPTH) {
                    if (current === rootId) return true;
                    const node = locationMap.get(current);
                    current = node ? node.parent_id : null;
                    depth++;
                }
                return false;
            };

            let someMovedIn = false;
            const toRemoveIDs: string[] = [];

            for (const item of updatedItems) {
                const isNowInView =
                    selectedFolderIds.length === 0 ||
                    selectedFolderIds.some(folderId =>
                        recursive
                            ? isChildOf(item.folder_id, folderId)
                            : item.folder_id === folderId
                    );

                const wasKnown = currentItems.some(i => i.id === item.id);

                if (isNowInView) {
                    if (wasKnown) {
                        setLibraryState(
                            'items',
                            i => i.id === item.id,
                            prev => ({
                                ...prev,
                                path: item.path,
                                filename: item.filename,
                                modified_at: item.modified_at,
                                folder_id: item.folder_id
                            })
                        );
                    } else {
                        someMovedIn = true;
                    }
                } else if (wasKnown) {
                    toRemoveIDs.push(item.id);
                }
            }

            if (toRemoveIDs.length > 0) {
                const removeSet = new Set(toRemoveIDs);
                setLibraryState('items', items => items.filter(i => !removeSet.has(i.id)));
            }

            if (someMovedIn) {
                libraryActions.refreshAssets(false);
            }
        }
    },

    /** Reloads the viewport once after an atomic folder-tree removal. */
    refreshAfterFolderRemoval: async (): Promise<void> => {
        if (refreshDebounceTimer) {
            clearTimeout(refreshDebounceTimer);
            refreshDebounceTimer = null;
        }
        await libraryActions.refreshAssets(false);
    },

    addLocation: async () => {
        try {
            const selectedPath = await open({
                directory: true,
                multiple: false,
                title: 'Select Folder to Add'
            });

            if (selectedPath && !Array.isArray(selectedPath)) {
                await dbAddLocation(selectedPath);
                await metadataActions.loadLocations();
                await tauriService.startIndexing({ path: selectedPath });
                await libraryActions.refreshAssets(true);
                return { success: true, path: selectedPath };
            }
            return { success: false };
        } catch (err) {
            console.error('Failed to add location:', err);
            return { success: false, error: err };
        }
    },

    removeLocation: async (locationId: number | string) => {
        try {
            await invoke('remove_location', { folderId: String(locationId) });

            return { success: true };
        } catch (err) {
            console.error('Failed to remove location:', err);
            return { success: false, error: err };
        }
    },

    applyTagToAssets: async (
        assetIds: string[],
        tagId: string
    ): Promise<ActionResult<{ tagName: string; count: number }>> => {
        if (assetIds.length === 0) {
            return {
                success: false,
                error: { code: ErrorCode.VALIDATION_ERROR, message: 'No items provided' }
            };
        }

        try {
            // Reusing updateAssetsTags which properly triggers notifications and emits events
            await metadataActions.updateAssetsTags(assetIds, [tagId], 'merge');

            const tagName = metadataState.tags.find(tag => tag.id === tagId)?.name || 'Tag';
            return {
                success: true,
                data: {
                    tagName,
                    count: assetIds.length
                }
            };
        } catch (error) {
            console.error('Failed to apply tags to assets:', error);
            return {
                success: false,
                error: { code: ErrorCode.IO_ERROR, message: 'Failed to apply tags' }
            };
        }
    },

    applyTagToSelection: async (
        tagId: string
    ): Promise<ActionResult<{ tagName: string; count: number }>> => {
        return libraryActions.applyTagToAssets(selectionState.selectedIds, tagId);
    },

    applyTagToTarget: async (
        tagId: string,
        targetAssetId: string
    ): Promise<ActionResult<{ tagName: string; count: number }>> => {
        let targetIds = [targetAssetId];
        if (selectionState.selectedIds.includes(targetAssetId)) {
            targetIds = [...selectionState.selectedIds];
        }
        return libraryActions.applyTagToAssets(targetIds, tagId);
    },

    removeTagFromSelection: async (tagId: string): Promise<ActionResult> => {
        const selectedIds = selectionState.selectedIds;
        if (selectedIds.length === 0) {
            return {
                success: false,
                error: { code: ErrorCode.VALIDATION_ERROR, message: 'No items selected' }
            };
        }

        try {
            // Reusing updateAssetsTags to properly emit events and bump tagUpdateVersion
            await metadataActions.updateAssetsTags(selectedIds, [tagId], 'remove');
            return { success: true, data: undefined };
        } catch (error) {
            console.error('Failed to remove tags from selection:', error);
            return {
                success: false,
                error: { code: ErrorCode.IO_ERROR, message: 'Failed to remove tags' }
            };
        }
    }
};

setRefreshAssetsHandler(clear => libraryActions.refreshAssets(clear));

eventBus.on('metadata:changed', e => {
    if (e.type === 'tag') {
        const isFilteringByTags =
            filterState.filterUntagged ||
            filterState.filterHasTags ||
            filterState.selectedTags.length > 0;
        if (isFilteringByTags) {
            libraryActions.refreshAssets(false);
        }
    }
});
