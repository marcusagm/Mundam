import type { DragItem } from '../../../../core/dnd/dnd-core';
import type { TreeNode } from '../types';

/** Builds typed tree payloads without treating folders as assets. */
export function createTreeDragItem(dragType: string, node: TreeNode): DragItem | null {
    const data = node.data as { path?: string; parent_id?: string | null } | undefined;
    const identifier = String(node.id);
    switch (dragType) {
        case 'FOLDER':
            return {
                type: 'FOLDER',
                payload: { id: identifier, name: node.label, path: data?.path || '' }
            };
        case 'TAG':
            return {
                type: 'TAG',
                payload: { id: identifier, name: node.label, parent_id: data?.parent_id }
            };
        case 'ASSET':
            return {
                type: 'ASSET',
                payload: {
                    id: identifier,
                    ids: [identifier],
                    filename: node.label,
                    path: data?.path || ''
                }
            };
        default:
            return null;
    }
}
