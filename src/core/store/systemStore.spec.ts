import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    listeners: new Map<string, (event: { payload: unknown }) => void>(),
    refreshAssets: vi.fn().mockResolvedValue(undefined),
    loadStats: vi.fn().mockResolvedValue(undefined),
    loadLocations: vi.fn().mockResolvedValue(undefined)
}));

vi.mock('@tauri-apps/api/event', () => ({
    listen: async (name: string, handler: (event: { payload: unknown }) => void) => {
        mocks.listeners.set(name, handler);
        return vi.fn();
    }
}));
vi.mock('@tauri-apps/api/webviewWindow', () => ({ WebviewWindow: {} }));
vi.mock('../../lib/db', () => ({ initDb: vi.fn(), addLocation: vi.fn() }));
vi.mock('../../lib/hls-player', () => ({ initStreamingToken: vi.fn() }));
vi.mock('../tauri/services', () => ({ tauriService: {} }));
vi.mock('./formatStore', () => ({ formatState: { formats: [] } }));
vi.mock('./metadata', () => ({
    metadataState: { locations: [] },
    metadataActions: {
        loadLocations: mocks.loadLocations,
        loadTags: vi.fn(),
        loadSmartFolders: vi.fn(),
        loadStats: mocks.loadStats
    }
}));
vi.mock('./library', () => ({
    libraryActions: { refreshAssets: mocks.refreshAssets }
}));
vi.mock('./library/libraryEvents', () => ({ handleLibraryBatchChange: vi.fn() }));
import { indexingOutcome, progress, systemActions } from './systemStore';

describe('indexing lifecycle feedback', () => {
    beforeAll(async () => {
        await systemActions.initialize();
    });

    beforeEach(() => {
        vi.clearAllMocks();
        systemActions.updateProgress({ total: 10, processed: 4, current_file: 'file' });
    });

    it('clears progress and reports failure without reporting success', async () => {
        mocks.listeners.get('indexer:failed')?.({ payload: { library_id: 'root' } });
        await Promise.resolve();
        expect(progress()).toBeNull();
        expect(indexingOutcome()).toBe('failed');
        expect(mocks.refreshAssets).toHaveBeenCalledWith(true);
        expect(mocks.loadStats).toHaveBeenCalledOnce();
        expect(mocks.loadLocations).toHaveBeenCalledOnce();
    });

    it('clears progress and reports completion without reporting failure', () => {
        mocks.listeners.get('indexer:complete')?.({ payload: 10 });
        expect(progress()).toBeNull();
        expect(indexingOutcome()).toBe('completed');
    });

    it('logs a failed refresh after indexing', async () => {
        const error = new Error('refresh failed');
        const log = vi.spyOn(console, 'error').mockImplementation(() => {});
        mocks.refreshAssets.mockRejectedValueOnce(error);
        mocks.listeners.get('indexer:failed')?.({ payload: { library_id: 'root' } });
        await vi.waitFor(() =>
            expect(log).toHaveBeenCalledWith('Failed to refresh library after indexing:', error)
        );
        log.mockRestore();
    });
});
