import { reconcile } from 'solid-js/store';
import { invokeCommand as invoke } from '../../../lib/api';
import { tagService } from '../../../lib/tags';
import { filterState } from '../filter';
import { metadataActions } from '../metadata';
import { libraryStateInternal } from './libraryState';

const { setLibraryState } = libraryStateInternal;

let refreshAssetsHandler: ((clear?: boolean) => Promise<void>) | null = null;
export function setRefreshAssetsHandler(handler: (clear?: boolean) => Promise<void>) {
    refreshAssetsHandler = handler;
}

export const itemActions = {
    updateItemRating: async (id: string, rating: number) => {
        try {
            setLibraryState('items', i => i.id === id, 'rating', rating);
            await tagService.updateAssetRating(id, rating);
        } catch (err) {
            console.error(`Failed to update rating for ${id}:`, err);
        }
    },

    updateItemNotes: async (id: string, notes: string) => {
        try {
            setLibraryState('items', i => i.id === id, 'notes', notes);
            await tagService.updateAssetNotes(id, notes);
        } catch (err) {
            console.error(`Failed to update notes for ${id}:`, err);
        }
    },

    toggleItemFavorite: async (id: string) => {
        try {
            // Optimistic update
            setLibraryState(
                'items',
                i => i.id === id,
                'is_favorite',
                (prev: boolean | undefined) => !prev
            );
            await invoke('toggle_favorite', { assetId: id });

            if (filterState.filterFavorites) {
                refreshAssetsHandler?.(false);
            }
            metadataActions.loadStats();
        } catch (err) {
            console.error(`Failed to toggle favorite for ${id}:`, err);
            // Revert optimistic update
            setLibraryState(
                'items',
                i => i.id === id,
                'is_favorite',
                (prev: boolean | undefined) => !prev
            );
        }
    },

    moveToTrashAssets: async (ids: string[]) => {
        for (const assetId of ids) {
            try {
                await invoke('move_to_trash', { assetId });
            } catch (err) {
                console.error(`Failed to move to trash ${assetId}:`, err);
            }
        }
        refreshAssetsHandler?.(false);
        metadataActions.loadStats();
    },

    restoreFromTrashAssets: async (ids: string[]) => {
        for (const assetId of ids) {
            try {
                await invoke('restore_from_trash', { assetId });
            } catch (err) {
                console.error(`Failed to restore ${assetId}:`, err);
            }
        }
        refreshAssetsHandler?.(false);
        metadataActions.loadStats();
    },

    updateThumbnail: (id: string, path: string) => {
        setLibraryState('items', item => item.id === id, 'thumbnail_path', path);
    },

    refreshItem: async (id: string) => {
        try {
            const asset = await invoke<import('../../../types').AssetItem>('get_asset', { id });
            if (asset) {
                setLibraryState('items', item => item.id === id, reconcile(asset));
            }
        } catch (err) {
            console.error(`Failed to refresh item ${id}:`, err);
        }
    },

    setThumbnailPriority: async (ids: string[]) => {
        try {
            await invoke('set_thumbnail_priority', { ids });
        } catch (err) {
            console.error('Failed to set thumbnail priority:', err);
        }
    }
};
