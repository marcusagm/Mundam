import { afterEach, describe, expect, it } from 'vitest';
import { parseDragItem, readDragItem } from './parseDragItem';
import { setDragItem } from './dnd-core';

afterEach(() => setDragItem(null));
describe('untrusted drag payloads', () => {
    it.each([
        null,
        {},
        { type: 'FOLDER', payload: {} },
        { type: 'ASSET', payload: { id: 'a', ids: [3], filename: 'a', path: '/a' } }
    ])('rejects incomplete data %j', value => {
        expect(parseDragItem(value)).toBeNull();
    });
    it('does not fall back to internal state when supplied JSON is malformed', () => {
        setDragItem({ type: 'TAG', payload: { id: 'a', name: 'A' } });
        expect(readDragItem({ getData: () => '{broken' } as unknown as DataTransfer)).toBeNull();
    });
    it('accepts a valid external payload without internal state', () => {
        const item = { type: 'FOLDER', payload: { id: 'a', name: 'A', path: '/a' } };
        expect(
            readDragItem({ getData: () => JSON.stringify(item) } as unknown as DataTransfer)
        ).toEqual(item);
    });
});
