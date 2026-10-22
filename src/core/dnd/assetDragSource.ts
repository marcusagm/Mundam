/**
 * Asset Drag Source Directive
 *
 * Simplified directive that ONLY handles drag start and ghost creation.
 * Drop handling is now done at the container level via ViewportDropZone.
 *
 * This separation allows virtualization to work correctly since we don't
 * need to attach drop handlers to each item.
 */

import { createEffect, onCleanup } from 'solid-js';
import { setDragItem, DragItem } from './dnd-core';
import { createDragGhost } from './ghost';
import { AssetItem } from '../../types';

export interface AssetDragSourceParams {
    /** Unique ID of this item */
    id: string;
    /** Path to the asset file */
    path: string;
    /** Thumbnail path for ghost image */
    thumbnailPath: string | null;
    /** Whether this item is currently selected */
    isSelected: boolean;
    /** Get all currently selected IDs (for multi-drag) */
    getSelectedIds: () => string[];
    /** Get item info by ID (for ghost creation) */
    getItemInfo: (id: string) => { path: string; thumbnail_path: string | null } | undefined;
    /** Allows dragging only while true (defaults to always enabled) */
    enabled?: boolean;
}

/**
 * SolidJS Directive for Asset Drag Source.
 * Only handles dragstart and dragend - no drop target logic.
 */
export function assetDragSource(el: HTMLElement, accessor: () => AssetDragSourceParams) {
    const handleDragStart = (e: DragEvent) => {
        if (accessor().enabled === false) {
            e.preventDefault();
            return;
        }
        e.stopPropagation();
        if (!e.dataTransfer) return;

        const params = accessor();
        const { id, isSelected, getSelectedIds, getItemInfo } = params;

        // Determine which IDs to drag
        let ids: string[] = [id];
        if (isSelected) {
            const selectedIds = getSelectedIds();
            if (selectedIds.includes(id)) {
                ids = [...selectedIds];
            }
        }

        // Set drag data
        const data: DragItem = {
            type: 'ASSET',
            payload: {
                id,
                ids,
                filename: params.id, // We don't have filename in params, but it's used for UI feedback
                path: params.path,
                thumbnail_path: params.thumbnailPath
            }
        };
        setDragItem(data);

        e.dataTransfer.effectAllowed = 'copyMove';
        e.dataTransfer.setData('application/json', JSON.stringify(data));

        // Build paths for external drag
        const validPaths: string[] = [];
        const cleanPaths: string[] = [];

        ids.forEach(itemId => {
            const info = getItemInfo(itemId);
            if (info?.path) {
                validPaths.push(`file://${info.path}`);
                cleanPaths.push(info.path);
            }
        });

        if (validPaths.length > 0) {
            e.dataTransfer.setData('text/uri-list', validPaths.join('\r\n'));
            e.dataTransfer.setData('text/plain', cleanPaths.join('\n'));
        }

        // Create ghost image
        const ghostItems = ids.map(itemId => {
            const info = getItemInfo(itemId);
            return {
                id: itemId,
                path: info?.path || '',
                thumbnail_path: info?.thumbnail_path || null
            };
        });

        const ghost = createDragGhost(ghostItems as unknown as AssetItem[]);
        e.dataTransfer.setDragImage(ghost, 0, 0);

        // Cleanup ghost after render
        setTimeout(() => {
            if (ghost.parentNode) {
                document.body.removeChild(ghost);
            }
        }, 0);
    };

    const handleDragEnd = () => {
        setDragItem(null);
    };

    // Attach listeners
    createEffect(() => {
        el.setAttribute('draggable', accessor().enabled === false ? 'false' : 'true');
    });
    el.addEventListener('dragstart', handleDragStart);
    el.addEventListener('dragend', handleDragEnd);

    onCleanup(() => {
        el.removeEventListener('dragstart', handleDragStart);
        el.removeEventListener('dragend', handleDragEnd);
    });
}
