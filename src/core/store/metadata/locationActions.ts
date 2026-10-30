import { metadataState, setMetadataState } from './metadataState';
import { getLocations } from '../../../lib/db';
import { tagService } from '../../../lib/tags';
import { computeStatsFromBatchChange } from '../statsHelpers';
import { type BatchChangePayload } from '../library';

/** Check if some added items belong to unknown folders */
function hasUnknownFolders(added: BatchChangePayload['added'], knownIds: Set<string>): boolean {
    if (!added) return false;
    return added.some(item => item.folder_id && !knownIds.has(item.folder_id));
}

// Minimal definition of refreshAll to avoid circular dependency
let tagRefs = { loadTags: async () => {} };
let searchRefs = { loadSmartFolders: async () => {} };
export function initLocationRefs(
    tags: { loadTags: () => Promise<void> },
    searches: { loadSmartFolders: () => Promise<void> }
) {
    tagRefs = tags;
    searchRefs = searches;
}

let refreshDebounceTimer: ReturnType<typeof setTimeout> | null = null;

export const locationActions = {
    loadLocations: async (failOnError = false) => {
        try {
            const locations = await getLocations();
            setMetadataState(
                'locations',
                locations.map(l => ({ ...l, is_root: l.parent_id === null }))
            );
        } catch (error) {
            console.error('Failed to load locations:', error);
            if (failOnError) throw error;
        }
    },

    loadStats: async () => {
        try {
            const stats = await tagService.getLibraryStats();
            const tagMap = new Map();
            stats.tag_counts.forEach(tagCount => tagMap.set(tagCount.tag_id, tagCount.count));

            const folderMap = new Map();
            stats.folder_counts.forEach(folderCount =>
                folderMap.set(folderCount.folder_id, folderCount.count)
            );

            const folderRecursiveMap = new Map();
            if (stats.folder_counts_recursive) {
                stats.folder_counts_recursive.forEach(folderCount =>
                    folderRecursiveMap.set(folderCount.folder_id, folderCount.count)
                );
            }

            setMetadataState('libraryStats', {
                total_assets: stats.total_assets,
                total_folders: stats.total_folders,
                total_tags: stats.total_tags,
                total_size_bytes: stats.total_size_bytes,
                untagged_assets: stats.untagged_assets,
                has_tags_assets: stats.has_tags_assets,
                favorite_assets: stats.favorite_assets,
                trash_assets: stats.trash_assets,
                smart_folders: stats.smart_folders,
                duplicate_assets: stats.duplicate_assets,
                tag_counts: tagMap,
                folder_counts: folderMap,
                folder_counts_recursive: folderRecursiveMap,
                family_stats: stats.family_stats ?? []
            });
        } catch (error) {
            console.error('Failed to load library stats:', error);
        }
    },

    refreshAll: async () => {
        await Promise.all([
            tagRefs.loadTags(),
            locationActions.loadLocations(),
            locationActions.loadStats(),
            searchRefs.loadSmartFolders()
        ]);
    },

    handleBatchChange: (payload: BatchChangePayload) => {
        const knownIds = new Set(metadataState.locations.map(location => location.id));
        let needsRefresh = payload.needs_refresh ?? false;

        if (hasUnknownFolders(payload.added, knownIds)) {
            needsRefresh = true;
        }

        setMetadataState('libraryStats', stats => {
            const result = computeStatsFromBatchChange(
                stats,
                payload,
                metadataState.locations,
                knownIds
            );
            needsRefresh = needsRefresh || result.needsRefresh;
            return result.newStats;
        });

        if (needsRefresh) {
            if (refreshDebounceTimer) {
                clearTimeout(refreshDebounceTimer);
            }
            refreshDebounceTimer = setTimeout(() => {
                locationActions.refreshAll();
                refreshDebounceTimer = null;
            }, 500);
        }
    }
};
