import { invokeCommand as invoke } from './api';
import {
    type SearchCriteria,
    type AssetFilter,
    type UpdateTagsPayload,
    type PaginatedAssetsDto
} from '../types';

export interface Tag {
    id: string;
    name: string;
    parent_id: string | null;
    color: string | null;
    order_index: number;
}

export interface FamilyStat {
    family: string;
    count: number;
    total_size_bytes: number;
}

export interface LibraryStats {
    total_assets: number;
    total_folders: number;
    total_tags: number;
    total_size_bytes: number;
    untagged_assets: number;
    has_tags_assets: number;
    favorite_assets: number;
    trash_assets: number;
    smart_folders: number;
    duplicate_assets: number;
    tag_counts: { tag_id: string; count: number }[];
    folder_counts: { folder_id: string; count: number }[];
    folder_counts_recursive: { folder_id: string; count: number }[];
    family_stats: FamilyStat[];
}

export const tagService = {
    getAllTags: async (): Promise<Tag[]> => {
        return await invoke('list_tags');
    },

    getLibraryStats: async (): Promise<LibraryStats> => {
        return await invoke('get_library_stats'); // Assuming V2 keeps this unchanged
    },

    getTagsForAsset: async (assetId: string): Promise<Tag[]> => {
        return await invoke('get_tags_for_asset', { assetId: assetId }); // TBD V2 equivalent
    },

    searchAssets: async (
        criteria: SearchCriteria,
        page: number = 1,
        pageSize: number = 30
    ): Promise<PaginatedAssetsDto> => {
        return await invoke('search_assets', {
            criteria,
            page: { page, pageSize }
        });
    },

    getAssets: async (
        filter: AssetFilter,
        page: number = 1,
        pageSize: number = 30
    ): Promise<PaginatedAssetsDto> => {
        return await invoke('get_assets', {
            filter,
            page: { page, pageSize }
        });
    },

    updateAssetTags: async (payload: UpdateTagsPayload): Promise<void> => {
        return await invoke('update_asset_tags', { payload });
    },

    updateAssetRating: async (id: string, rating: number): Promise<void> => {
        return await invoke('update_asset_rating', { payload: { assetId: id, rating } });
    },

    updateAssetNotes: async (id: string, notes: string): Promise<void> => {
        return await invoke('update_asset_notes', { payload: { assetId: id, notes } });
    },

    createTag: async (
        name: string,
        parentId?: string | null,
        color?: string | null
    ): Promise<Tag> => {
        return await invoke('create_tag', { name, parentId, color });
    },

    updateTag: async (
        id: string,
        name?: string | null,
        color?: string | null,
        parentId?: string | null,
        orderIndex?: number | null
    ): Promise<void> => {
        return await invoke('update_tag', { id, name, color, parentId, orderIndex });
    },

    deleteTag: async (id: string): Promise<void> => {
        return await invoke('delete_tag', { id });
    },

    addTagsToAssetsBatch: async (assetIds: string[], tagIds: string[]): Promise<void> => {
        return await invoke('add_tags_to_assets_batch', { payload: { assetIds, tagIds } });
    },

    removeTagsFromAssetsBatch: async (assetIds: string[], tagIds: string[]): Promise<void> => {
        return await invoke('remove_tags_from_assets_batch', { payload: { assetIds, tagIds } });
    },

    replaceTagsForAssetsBatch: async (assetIds: string[], tagIds: string[]): Promise<void> => {
        // V2 updateAssetTags does not inherently "replace", it just adds/removes. A true replace would need
        // backend support or explicit removing all then adding the new ones. For now, adapting to API.
        // Assuming we need a backend change or this is a destructive operation.
        return await invoke('replace_asset_tags', { assetIds, tagIds }); // Placeholder for true "replace" handling if implemented, or we map it to updateAssetTags.
    },

    getAssetExif: async (assetId?: string, path?: string): Promise<Record<string, string>> => {
        return await invoke('get_asset_exif', { assetId, path });
    }
};
