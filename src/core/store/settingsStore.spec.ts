import { beforeEach, describe, expect, it, vi } from 'vitest';
import { settingsActions, processingConcurrency, thumbnailThreads } from './settingsStore';

const backend = vi.hoisted(() => ({
    getSetting: vi.fn(),
    setSetting: vi.fn(),
    getCacheStats: vi.fn()
}));
vi.mock('../tauri/services', () => ({ tauriService: backend }));

beforeEach(async () => {
    vi.resetAllMocks();
    backend.getSetting.mockImplementation(async (key: string) => {
        if (key === 'processing_concurrency') return '4';
        if (key === 'thumbnail_threads') return '2';
        return null;
    });
    backend.getCacheStats.mockResolvedValue({ file_count: 0, size_bytes: 0 });
    backend.setSetting.mockResolvedValue(undefined);
    await settingsActions.initialize();
});

describe('live processing settings', () => {
    it('keeps acknowledged limits until persistence succeeds', async () => {
        let complete!: () => void;
        backend.setSetting.mockImplementation(
            () =>
                new Promise<void>(resolve => {
                    complete = resolve;
                })
        );
        const saving = settingsActions.updateSettings({ processingConcurrency: 1 });
        expect(processingConcurrency()).toBe(4);
        expect(backend.setSetting).toHaveBeenCalledWith('processing_concurrency', '1');
        complete();
        expect((await saving).success).toBe(true);
        expect(processingConcurrency()).toBe(1);
    });

    it('retains shared and thumbnail limits after a failed write', async () => {
        const logging = vi.spyOn(console, 'error').mockImplementation(() => undefined);
        backend.setSetting.mockRejectedValue(new Error('private storage failure'));
        expect(
            (
                await settingsActions.updateSettings({
                    processingConcurrency: 1,
                    thumbnailThreads: 1
                })
            ).success
        ).toBe(false);
        expect(processingConcurrency()).toBe(4);
        expect(thumbnailThreads()).toBe(2);
        logging.mockRestore();
    });

    it('validates whole-number bounds and persists automatic mode', async () => {
        for (const processingConcurrency of [-1, 9, 1.5]) {
            expect((await settingsActions.updateSettings({ processingConcurrency })).success).toBe(
                false
            );
        }
        expect((await settingsActions.updateSettings({ thumbnailThreads: 1.5 })).success).toBe(
            false
        );
        expect(backend.setSetting).not.toHaveBeenCalled();
        expect(
            (
                await settingsActions.updateSettings({
                    processingConcurrency: 0,
                    thumbnailThreads: 0
                })
            ).success
        ).toBe(true);
        expect(processingConcurrency()).toBe(0);
        expect(thumbnailThreads()).toBe(0);
    });
});
