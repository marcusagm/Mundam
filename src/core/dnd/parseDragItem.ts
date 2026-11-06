import { currentDragItem, type DragItem } from './dnd-core';

import { z } from 'zod';

const identifier = z.union([z.string().min(1), z.number().finite()]);
const dragItemSchema = z.discriminatedUnion('type', [
    z.object({
        type: z.literal('TAG'),
        payload: z.object({
            id: identifier,
            name: z.string(),
            parent_id: identifier.nullish(),
            color: z.string().nullish()
        })
    }),
    z.object({
        type: z.literal('FOLDER'),
        payload: z.object({ id: z.string().min(1), name: z.string(), path: z.string().min(1) })
    }),
    z.object({
        type: z.literal('ASSET'),
        payload: z.object({
            id: z.string().min(1),
            ids: z.array(z.string().min(1)).min(1),
            filename: z.string(),
            path: z.string().min(1),
            thumbnail_path: z.string().nullish()
        })
    })
]);

/** Validates untrusted browser data before a domain drop handler runs.
 * @param value - Decoded external value.
 * @returns Validated item or null for unsupported/incomplete payloads.
 * @example parseDragItem({ type: 'TAG', payload: { id: 'tag', name: 'Tag' } });
 */
export function parseDragItem(value: unknown): DragItem | null {
    const result = dragItemSchema.safeParse(value);
    return result.success ? result.data : null;
}

/** Resolves one consistent payload policy for row, group and root drops.
 * @param transfer - Native transfer, unavailable in some synthetic events.
 * @returns Validated JSON, or validated internal state only when no JSON was supplied.
 * @example readDragItem(null);
 */
export function readDragItem(transfer: DataTransfer | null | undefined): DragItem | null {
    const raw = transfer?.getData('application/json');
    if (!raw) return parseDragItem(currentDragItem());
    try {
        return parseDragItem(JSON.parse(raw));
    } catch {
        return null;
    }
}
