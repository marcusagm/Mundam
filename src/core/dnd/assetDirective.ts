import { onCleanup, createEffect, on } from 'solid-js';
import { AssetItem } from '../../types';
import {
    dndRegistry,
    setDragItem,
    currentDragItem,
    setDropTargetId,
    currentDropTargetId,
    DragItem
} from './dnd-core';
import { createDragGhost } from './ghost';

export interface AssetDnDParams {
    item: AssetItem;
    selected: boolean;
    selectedIds: string[];
    allItems: AssetItem[];
}

/**
 * SolidJS Directive for Asset Drag and Drop.
 * Encapsulates complex drag ghost creation, selection-aware dragging,
 * and drop target logic for tags and assets.
 */
export function assetDnD(el: HTMLElement, accessor: () => AssetDnDParams) {
    // Local reactive state for this specific element
    let dragCounter = 0;

    // Reset internal state if the element is recycled for a different item (virtualization)
    createEffect(
        on(
            () => accessor().item.id,
            () => {
                dragCounter = 0;
            }
        )
    );

    // Sync class with GLOBAL reactive target state
    // This solves the virtualization bug where reused DOM nodes would keep old highlights
    createEffect(() => {
        const targetId = currentDropTargetId();
        const params = accessor();
        const myId = params.item.id;

        if (targetId !== null && targetId === myId) {
            el.classList.add('drop-target-active');
        } else {
            el.classList.remove('drop-target-active');
        }
    });

    // Auto-cleanup when global dragging ends anywhere
    createEffect(() => {
        if (!currentDragItem()) {
            dragCounter = 0;
            // No need to manually clear classes here as the effect above handles it
            // once currentDropTargetId is reset globally
        }
    });

    const handleDragStart = (e: DragEvent) => {
        e.stopPropagation();
        if (!e.dataTransfer) return;

        const { item, selected, selectedIds, allItems } = accessor();

        let ids: string[] = [item.id];
        if (selected && selectedIds.includes(item.id)) {
            ids = [...selectedIds] as string[];
        }

        const data: DragItem = {
            type: 'ASSET',
            payload: {
                id: item.id,
                ids,
                filename: item.filename,
                path: item.path,
                thumbnail_path: item.thumbnail_path
            }
        };
        setDragItem(data);

        e.dataTransfer.effectAllowed = 'copyMove';
        e.dataTransfer.setData('application/json', JSON.stringify(data));

        const draggedItems: AssetItem[] = [];
        const validPaths: string[] = [];
        const cleanPaths: string[] = [];

        ids.forEach(id => {
            const found = allItems.find(i => i.id === id);
            if (found) {
                draggedItems.push(found);
                if (found.path) {
                    validPaths.push(`file://${found.path}`);
                    cleanPaths.push(found.path);
                }
            }
        });

        if (validPaths.length > 0) {
            e.dataTransfer.setData('text/uri-list', validPaths.join('\r\n'));
            e.dataTransfer.setData('text/plain', cleanPaths.join('\n'));
        }

        const ghost = createDragGhost(draggedItems);
        e.dataTransfer.setDragImage(ghost, 0, 0);

        setTimeout(() => {
            if (ghost.parentNode) {
                document.body.removeChild(ghost);
            }
        }, 0);
    };

    const handleDragEnd = () => {
        setDragItem(null);
        setDropTargetId(null);
        dragCounter = 0;
    };

    const handleDragEnter = (e: DragEvent) => {
        e.preventDefault();
        dragCounter++;

        const dragging = currentDragItem();
        const strategy = dndRegistry.get('ASSET');
        if (strategy && strategy.onDragOver && dragging && strategy.onDragOver(dragging)) {
            if (e.dataTransfer) {
                const allowed = e.dataTransfer.effectAllowed;
                e.dataTransfer.dropEffect = allowed === 'move' ? 'move' : 'copy';
            }
            if (dragCounter === 1) {
                setDropTargetId(accessor().item.id);
            }
        }
    };

    const handleDragOver = (e: DragEvent) => {
        e.preventDefault();
        const dragging = currentDragItem();
        if (!dragging) return;

        const strategy = dndRegistry.get('ASSET');
        if (strategy && strategy.onDragOver && strategy.onDragOver(dragging)) {
            if (e.dataTransfer) {
                const allowed = e.dataTransfer.effectAllowed;
                e.dataTransfer.dropEffect = allowed === 'move' ? 'move' : 'copy';
            }
            // Ensure target is set if enter was missed or ID changed due to virtualization
            const myId = accessor().item.id;
            if (currentDropTargetId() !== myId) {
                setDropTargetId(myId);
            }
        }
    };

    const handleDragLeave = () => {
        dragCounter = Math.max(0, dragCounter - 1);
        if (dragCounter === 0) {
            if (currentDropTargetId() === accessor().item.id) {
                setDropTargetId(null);
            }
        }
    };

    const handleDrop = async (e: DragEvent) => {
        e.preventDefault();
        e.stopPropagation();
        dragCounter = 0;
        setDropTargetId(null);

        try {
            let data: DragItem | null = null;
            const json = e.dataTransfer?.getData('application/json');
            if (json) {
                try {
                    data = JSON.parse(json);
                } catch {
                    // ignore JSON parse error
                }
            }
            if (!data) {
                data = currentDragItem();
            }

            const strategy = dndRegistry.get('ASSET');
            if (data && strategy && strategy.accepts(data)) {
                const { item } = accessor();
                await strategy.onDrop(data, item.id);
            }
        } catch (err) {
            console.error('Drop failed', err);
        } finally {
            setDragItem(null);
            setDropTargetId(null);
        }
    };

    // Attach listeners
    el.setAttribute('draggable', 'true');
    el.addEventListener('dragstart', handleDragStart);
    el.addEventListener('dragend', handleDragEnd);
    el.addEventListener('dragenter', handleDragEnter);
    el.addEventListener('dragover', handleDragOver);
    el.addEventListener('dragleave', handleDragLeave);
    el.addEventListener('drop', handleDrop);

    onCleanup(() => {
        el.removeEventListener('dragstart', handleDragStart);
        el.removeEventListener('dragend', handleDragEnd);
        el.removeEventListener('dragenter', handleDragEnter);
        el.removeEventListener('dragover', handleDragOver);
        el.removeEventListener('dragleave', handleDragLeave);
        el.removeEventListener('drop', handleDrop);
    });
}
