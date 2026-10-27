import { metadataState, setMetadataState } from './metadataState';
import { Tag, tagService } from '../../../lib/tags';
import { ActionResult, ErrorCode } from '../../types/actions';
import { eventBus } from '../../utils/eventBus';
import { metadataCache } from './cache';

// Using a similar method to avoid circular reference issues with locationActions.js
let locationRefs = { loadStats: async () => {} };
export function initTagRefs(locations: { loadStats: () => Promise<void> }) {
    locationRefs = locations;
}

export const tagActions = {
    /**
     * Notifies the system of a tag update.
     * @param options - Configuration for what needs refreshing.
     */
    notifyTagUpdate: (options: { stats?: boolean; assets?: boolean } = {}) => {
        const { stats = true, assets = true } = options;

        setMetadataState('tagUpdateVersion', version => version + 1);

        if (stats) locationRefs.loadStats();

        // Check if we need to refresh the library
        if (assets) {
            eventBus.emit('metadata:changed', { type: 'tag' });
        }
    },

    /**
     * Loads all tags from the service and updates the metadata state.
     *
     * @returns {Promise<void>}
     */
    loadTags: async () => {
        try {
            const tags = await tagService.getAllTags();
            // Sort by order_index primarily
            setMetadataState(
                'tags',
                tags.sort((a, b) => a.order_index - b.order_index)
            );
        } catch (error) {
            console.error('Failed to load tags:', error);
        }
    },

    /**
     * Creates a new tag and refreshes metadata.
     */
    createTag: async (
        name: string,
        parentId?: string | null,
        color?: string | null
    ): Promise<ActionResult<string>> => {
        try {
            const result = await tagService.createTag(name, parentId, color);
            // In V2, service returns a full Tag object.
            const id = result.id;

            await tagActions.loadTags();
            tagActions.notifyTagUpdate();
            return { success: true, data: id };
        } catch (error) {
            console.error('Failed to create tag:', error);
            return {
                success: false,
                error: { code: ErrorCode.IO_ERROR, message: 'Failed to create tag' }
            };
        }
    },

    /**
     * Updates an existing tag and refreshes metadata.
     */
    updateTag: async (
        itemId: string,
        name?: string | null,
        color?: string | null,
        parentId?: string | null,
        orderIndex?: number | null
    ): Promise<ActionResult> => {
        try {
            const finalName = name === null ? undefined : name;
            const finalColor = color === null ? undefined : color;
            const finalParentId = parentId === null ? undefined : parentId;
            const finalOrderIndex = orderIndex === null ? undefined : orderIndex;

            await tagService.updateTag(
                itemId,
                finalName,
                finalColor,
                finalParentId,
                finalOrderIndex
            );

            tagActions.applyTagStoreUpdates(itemId, name, color, parentId, orderIndex);
            tagActions.resolveTagNotificationType(name, parentId, orderIndex);

            return { success: true, data: undefined };
        } catch (error) {
            console.error('Failed to update tag:', error);
            return {
                success: false,
                error: { code: ErrorCode.IO_ERROR, message: 'Failed to update tag' }
            };
        }
    },

    /**
     * Applies local store updates after a tag modification.
     */
    applyTagStoreUpdates: (
        itemId: string,
        name?: string | null,
        color?: string | null,
        parentId?: string | null,
        orderIndex?: number | null
    ) => {
        const tagUpdates: Partial<Tag> = {};

        if (name !== null && name !== undefined) tagUpdates.name = name;
        if (color !== null && color !== undefined) tagUpdates.color = color;
        if (orderIndex !== null && orderIndex !== undefined) tagUpdates.order_index = orderIndex;
        if (parentId !== undefined) {
            tagUpdates.parent_id = parentId === null ? null : String(parentId);
        }

        setMetadataState('tags', (tag: Tag) => tag.id === itemId, tagUpdates);
    },

    /**
     * Determines and triggers the correct notification after a tag update.
     */
    resolveTagNotificationType: async (
        name?: string | null,
        parentId?: string | null,
        orderIndex?: number | null
    ) => {
        const isStructuralChange = parentId !== undefined || orderIndex !== undefined;

        if (isStructuralChange) {
            await tagActions.loadTags();
            tagActions.notifyTagUpdate({});
        } else {
            tagActions.notifyTagUpdate({
                stats: false,
                assets: name !== null && name !== undefined
            });
        }
    },

    deleteTagRecursive: async (id: string): Promise<ActionResult> => {
        try {
            const allTags = metadataState.tags;
            const toDelete = new Set<string>([id]);

            // Simple BFS to find all descendants
            const queue = [id];
            while (queue.length > 0) {
                const currentId = queue.shift()!;
                const children = allTags.filter(t => t.parent_id === currentId);
                for (const child of children) {
                    if (!toDelete.has(child.id)) {
                        toDelete.add(child.id);
                        queue.push(child.id);
                    }
                }
            }
            // Delete in chunks or parallel? Sequential for now to ensure consistency
            // if DB has constraints, though usually it's fine.
            for (const tagId of toDelete) await tagService.deleteTag(tagId);

            await tagActions.loadTags();
            tagActions.notifyTagUpdate();
            return { success: true, data: undefined };
        } catch (error) {
            console.error('Failed to delete tags recursively:', error);
            return {
                success: false,
                error: { code: ErrorCode.IO_ERROR, message: 'Failed to delete tag' }
            };
        }
    },

    /**
     * Reorders multiple tags in a single operation.
     */
    reorderTags: async (updates: { id: string; order: number }[]): Promise<ActionResult> => {
        try {
            // Apply updates sequentially to the DB
            await Promise.all(
                updates.map(u => tagService.updateTag(u.id, null, null, undefined, u.order))
            );
            await tagActions.loadTags();
            return { success: true, data: undefined };
        } catch (error) {
            console.error('Failed to reorder tags:', error);
            return {
                success: false,
                error: { code: ErrorCode.IO_ERROR, message: 'Failed to reorder tags' }
            };
        }
    },

    /**
     * Moves a tag to a new parent or reorders it among siblings.
     */
    moveTag: async (
        draggedTagId: string,
        targetTagId: string | null,
        position: 'before' | 'inside' | 'after'
    ): Promise<ActionResult> => {
        try {
            let allTags = metadataState.tags;
            let draggedTag = allTags.find(t => String(t.id) === String(draggedTagId));

            if (!draggedTag) {
                console.warn(`[tagActions] Tag ${draggedTagId} not found in state, refreshing...`);
                await tagActions.loadTags();
                allTags = metadataState.tags;
                draggedTag = allTags.find(t => String(t.id) === String(draggedTagId));
            }

            if (!draggedTag) {
                console.error(
                    `[tagActions] Failed to move tag: Tag ${draggedTagId} still not found after refresh.`
                );
                throw new Error('Dragged tag not found');
            }

            let newParentId: string | null = null;
            if (position === 'inside') {
                newParentId = targetTagId;
            } else if (targetTagId !== null) {
                const targetTag = allTags.find(t => t.id === targetTagId);
                newParentId = targetTag ? targetTag.parent_id : null;
            }

            // 2. Build new sibling list
            const siblings = allTags
                .filter(tag => tag.parent_id === newParentId && tag.id !== draggedTagId)
                .sort((a, b) => a.order_index - b.order_index || a.name.localeCompare(b.name));

            let insertIndex = siblings.length;
            if (position !== 'inside') {
                const targetIndex = siblings.findIndex(tag => tag.id === targetTagId);
                if (targetIndex !== -1) {
                    insertIndex = position === 'before' ? targetIndex : targetIndex + 1;
                }
            }

            siblings.splice(insertIndex, 0, draggedTag!);

            // 3. Create and execute updates
            const updates = siblings.map((tag, index) => {
                const newOrder = index * 100;
                const isDragged = tag.id === draggedTagId;

                // Only update if parent changed or order changed.
                // NOTE: We pass "0" for newParentId if it's null, as the Rust backend
                // normalizes "0" or "" as NULL in the database.
                if (isDragged || tag.order_index !== newOrder) {
                    return tagService.updateTag(
                        tag.id,
                        null,
                        null,
                        isDragged ? (newParentId ?? '0') : tag.parent_id,
                        newOrder
                    );
                }
                return Promise.resolve();
            });

            await Promise.all(updates);
            await tagActions.loadTags();

            return { success: true, data: undefined };
        } catch (error) {
            console.error('Failed to move tag:', error);
            return {
                success: false,
                error: { code: ErrorCode.IO_ERROR, message: 'Failed to move tag' }
            };
        }
    },

    /**
     * Batch updates tags for multiple assets.
     */
    updateAssetsTags: async (
        assetIds: string[],
        tagIds: string[],
        mode: 'merge' | 'replace' | 'remove'
    ): Promise<ActionResult> => {
        try {
            if (mode === 'merge') {
                await tagService.addTagsToAssetsBatch(assetIds, tagIds);
            } else if (mode === 'remove') {
                await tagService.removeTagsFromAssetsBatch(assetIds, tagIds);
            } else {
                await tagService.replaceTagsForAssetsBatch(assetIds, tagIds);
            }

            tagActions.notifyTagUpdate({ stats: true, assets: true });
            eventBus.emit('metadata:changed', { type: 'tag', ids: tagIds });

            return { success: true, data: undefined };
        } catch (error) {
            console.error('Batch tag update failed:', error);
            return {
                success: false,
                error: { code: ErrorCode.IO_ERROR, message: 'Batch tag update failed' }
            };
        }
    },

    /**
     * Batch updates metadata (rating, notes) for multiple assets.
     */
    updateAssetsMetadata: async (
        assetIds: string[],
        metadata: { rating?: number; notes?: string }
    ): Promise<ActionResult> => {
        try {
            const updates = assetIds.flatMap(id => {
                const results = [];
                if (metadata.rating !== undefined) {
                    results.push(tagService.updateAssetRating(id, metadata.rating));
                }
                if (metadata.notes !== undefined) {
                    results.push(tagService.updateAssetNotes(id, metadata.notes));
                }
                return results;
            });

            await Promise.all(updates);

            tagActions.notifyTagUpdate({ stats: false, assets: true });
            eventBus.emit('assets:metadata-updated', {
                assetIds: assetIds.map(String),
                fields: Object.keys(metadata)
            });

            return { success: true, data: undefined };
        } catch (error) {
            console.error('Batch metadata update failed:', error);
            return {
                success: false,
                error: { code: ErrorCode.IO_ERROR, message: 'Batch metadata update failed' }
            };
        }
    },

    /**
     * Retrieves EXIF/Technical metadata for an asset, utilizing the local cache.
     */
    getAssetExif: async (assetId: string, path: string): Promise<Record<string, string>> => {
        const cached = metadataCache.get<Record<string, string>>(String(assetId));
        if (cached) return cached;

        try {
            const exif = await tagService.getAssetExif(assetId, path);
            metadataCache.set(String(assetId), exif);
            return exif;
        } catch (error) {
            console.error(`Failed to load EXIF for asset ${assetId}:`, error);
            return {};
        }
    },

    /**
     * Retrieves the tags associated with a specific asset.
     */
    getAssetTags: async (assetId: string): Promise<Tag[]> => {
        try {
            return await tagService.getTagsForAsset(assetId);
        } catch (error) {
            console.error(`Failed to load tags for asset ${assetId}:`, error);
            return [];
        }
    }
};
