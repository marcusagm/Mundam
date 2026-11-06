import type { DragItem } from './dnd-core';
interface DragNode {
    id: string | number;
    label: string;
    data?: unknown;
}

/** Adapts navigation nodes to validated domain payloads.
 * @param dragType - Closed domain discriminant.
 * @param node - Display node with optional domain metadata.
 * @returns Domain payload or null for unsupported types.
 * @example createTreeDragItem('TAG', { id: 'tag', label: 'Tag' });
 */
export function createTreeDragItem(dragType: string, node: DragNode): DragItem | null {
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
