import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ThumbnailProcessingClient, type ThumbnailProcessingSnapshot } from './thumbnailProcessing';

const completed = (identifier: string): ThumbnailProcessingSnapshot => ({
    asset_id: identifier,
    state: 'succeeded',
    generation: 0,
    thumbnail_path: `${identifier}.webp`
});

describe('bounded thumbnail status transport', () => {
    let cleanups: (() => void)[];
    beforeEach(() => {
        vi.useFakeTimers();
        vi.setSystemTime(0);
        cleanups = [];
    });
    afterEach(() => {
        cleanups.forEach(cleanup => cleanup());
        vi.useRealTimers();
        vi.restoreAllMocks();
    });

    it('batches visible components, caps RPC size and stops polling completed work', async () => {
        const load = vi.fn(async (identifiers: string[]) => identifiers.map(completed));
        const client = new ThumbnailProcessingClient(load);
        for (let index = 0; index < 201; index++)
            cleanups.push(client.subscribe(`asset-${index}`, vi.fn()));
        await vi.advanceTimersByTimeAsync(65);
        expect(load).toHaveBeenCalledTimes(2);
        expect(load.mock.calls.map(([identifiers]) => identifiers.length)).toEqual([200, 1]);
        await vi.advanceTimersByTimeAsync(10000);
        expect(load).toHaveBeenCalledTimes(2);
    });

    it('shares subscriptions for duplicate cards and releases unmounted assets', async () => {
        const load = vi.fn(async (identifiers: string[]) => identifiers.map(completed));
        const client = new ThumbnailProcessingClient(load);
        const first = vi.fn();
        const second = vi.fn();
        cleanups.push(client.subscribe('same', first), client.subscribe('same', second));
        await vi.advanceTimersByTimeAsync(35);
        expect(load).toHaveBeenCalledExactlyOnceWith(['same']);
        expect(first).toHaveBeenLastCalledWith(completed('same'));
        expect(second).toHaveBeenLastCalledWith(completed('same'));
        const stop = client.subscribe('unmounted', vi.fn());
        stop();
        await vi.advanceTimersByTimeAsync(35);
        expect(load).toHaveBeenCalledTimes(1);
    });

    it('polls deferred retries, stops at failure and observes an explicit new request', async () => {
        let state: ThumbnailProcessingSnapshot['state'] = 'retry_wait';
        const listener = vi.fn();
        const load = vi.fn(async () => [{ ...completed('asset'), thumbnail_path: null, state }]);
        const client = new ThumbnailProcessingClient(load);
        cleanups.push(client.subscribe('asset', listener));
        await vi.advanceTimersByTimeAsync(35);
        state = 'failed';
        await vi.advanceTimersByTimeAsync(2000);
        expect(listener.mock.lastCall?.[0].state).toBe('failed');
        await vi.advanceTimersByTimeAsync(4000);
        expect(load).toHaveBeenCalledTimes(2);
        state = 'pending';
        client.invalidate('asset');
        await vi.advanceTimersByTimeAsync(35);
        expect(listener.mock.lastCall?.[0].state).toBe('pending');
        expect(load).toHaveBeenCalledTimes(3);
    });

    it('rejects a stale response from before an explicit invalidation', async () => {
        let resolveOld: ((snapshots: ThumbnailProcessingSnapshot[]) => void) | undefined;
        const load = vi.fn(
            async () =>
                new Promise<ThumbnailProcessingSnapshot[]>(resolve => {
                    resolveOld = resolve;
                })
        );
        const client = new ThumbnailProcessingClient(load);
        const listener = vi.fn();
        cleanups.push(client.subscribe('asset', listener));
        await vi.advanceTimersByTimeAsync(35);
        client.invalidate('asset');
        load.mockResolvedValueOnce([completed('asset')]);
        if (!resolveOld) throw new Error('Expected a pending batch');
        resolveOld([{ ...completed('asset'), state: 'skipped' }]);
        await vi.advanceTimersByTimeAsync(35);
        expect(listener.mock.calls.some(([snapshot]) => snapshot?.state === 'skipped')).toBe(false);
        expect(listener).toHaveBeenLastCalledWith(completed('asset'));
    });

    it('reports an unknown initial status on transport failure and recovers later', async () => {
        vi.spyOn(console, 'warn').mockImplementation(() => {});
        const load = vi
            .fn<() => Promise<ThumbnailProcessingSnapshot[]>>()
            .mockRejectedValueOnce(new Error('offline'))
            .mockResolvedValueOnce([completed('asset')]);
        const client = new ThumbnailProcessingClient(load);
        const listener = vi.fn();
        cleanups.push(client.subscribe('asset', listener));
        await vi.advanceTimersByTimeAsync(35);
        expect(listener.mock.lastCall?.[0].state).toBe('unknown');
        await vi.advanceTimersByTimeAsync(2000);
        expect(listener).toHaveBeenLastCalledWith(completed('asset'));
    });

    it.each(['pending', 'succeeded'] as const)(
        'preserves the last %s snapshot during invalidation and transport recovery',
        async state => {
            vi.spyOn(console, 'warn').mockImplementation(() => {});
            const snapshot = { ...completed('stable'), generation: 7, state };
            const recovered = { ...completed('stable'), generation: 8 };
            const load = vi
                .fn<() => Promise<ThumbnailProcessingSnapshot[]>>()
                .mockResolvedValueOnce([snapshot])
                .mockRejectedValueOnce(new Error('temporarily unavailable'))
                .mockResolvedValueOnce([recovered]);
            const client = new ThumbnailProcessingClient(load);
            const listener = vi.fn();
            cleanups.push(client.subscribe('stable', listener));
            await vi.advanceTimersByTimeAsync(35);
            client.invalidate('stable');
            expect(listener).toHaveBeenLastCalledWith(snapshot);
            await vi.advanceTimersByTimeAsync(35);
            expect(listener).toHaveBeenLastCalledWith(snapshot);
            await vi.advanceTimersByTimeAsync(2000);
            expect(listener).toHaveBeenLastCalledWith(recovered);
            expect(load).toHaveBeenCalledTimes(3);
        }
    );
});
