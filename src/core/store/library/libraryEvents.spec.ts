import { beforeEach, describe, expect, it, vi } from 'vitest';

const actions = vi.hoisted(() => ({
    refreshLibrary: vi.fn(async () => {}),
    refreshMetadata: vi.fn(async () => {}),
    updateLibrary: vi.fn(),
    updateMetadata: vi.fn()
}));

vi.mock('./libraryActions', () => ({
    libraryActions: {
        refreshAfterFolderRemoval: actions.refreshLibrary,
        handleBatchChange: actions.updateLibrary
    }
}));
vi.mock('../metadata', () => ({
    metadataActions: {
        refreshAfterFolderRemoval: actions.refreshMetadata,
        handleBatchChange: actions.updateMetadata
    }
}));

import { handleLibraryBatchChange } from './libraryEvents';

describe('committed folder removal events', () => {
    beforeEach(() => vi.clearAllMocks());

    it('refreshes each store once without incremental count changes or debounce scheduling', async () => {
        await handleLibraryBatchChange({ operation: 'folder_removal', needs_refresh: true });

        expect(actions.refreshLibrary).toHaveBeenCalledTimes(1);
        expect(actions.refreshMetadata).toHaveBeenCalledTimes(1);
        expect(actions.updateLibrary).not.toHaveBeenCalled();
        expect(actions.updateMetadata).not.toHaveBeenCalled();
    });

    it('preserves incremental processing for ordinary watcher changes', async () => {
        const payload = { removed: [{ id: 'asset', folder_id: 'folder', tag_ids: [] }] };
        await handleLibraryBatchChange(payload);

        expect(actions.updateLibrary).toHaveBeenCalledExactlyOnceWith(payload);
        expect(actions.updateMetadata).toHaveBeenCalledExactlyOnceWith(payload);
        expect(actions.refreshLibrary).not.toHaveBeenCalled();
        expect(actions.refreshMetadata).not.toHaveBeenCalled();
    });
});
