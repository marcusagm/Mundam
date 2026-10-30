import { beforeEach, describe, expect, it, vi } from 'vitest';

const invoke = vi.hoisted(() => vi.fn(async () => undefined));
vi.mock('../../../lib/api', () => ({ invokeCommand: invoke }));
vi.mock('../../../lib/tags', () => ({ tagService: {} }));
vi.mock('../filter', () => ({ filterState: {} }));
vi.mock('../metadata', () => ({ metadataActions: {} }));
vi.mock('./libraryState', () => ({ libraryStateInternal: { setLibraryState: vi.fn() } }));

import { itemActions } from './itemActions';

describe('processing viewport priorities', () => {
    beforeEach(() => vi.clearAllMocks());

    it('clears backend priorities when the viewport becomes empty', async () => {
        await itemActions.setThumbnailPriority([]);
        expect(invoke).toHaveBeenCalledExactlyOnceWith('set_thumbnail_priority', { ids: [] });
    });

    it('preserves the visible ordering when replacing priorities', async () => {
        await itemActions.setThumbnailPriority(['second', 'first']);
        expect(invoke).toHaveBeenCalledExactlyOnceWith('set_thumbnail_priority', {
            ids: ['second', 'first']
        });
    });
});
