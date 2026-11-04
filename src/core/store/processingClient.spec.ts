import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ProcessingClient, type ProcessingTransport } from './processingClient';
import type { ProcessingClientState, ProcessingSnapshot } from './processingTypes';

function snapshot(sequence = 1, paused = false): ProcessingSnapshot {
    return {
        instance_id: 'session',
        sequence,
        workers: [
            {
                worker: 'thumbnails',
                online: true,
                paused,
                active: 0,
                phase: null,
                scheduled: false,
                pending: 2,
                retry_wait: 0,
                failed: 0,
                skipped: 0,
                automatic_enabled: true
            }
        ],
        manual_scan: { is_running: false, phase: 'idle', processed: 0, total: 0 }
    };
}

function deferred<Value>() {
    let resolve!: (value: Value) => void;
    const promise = new Promise<Value>(complete => {
        resolve = complete;
    });
    return { promise, resolve };
}

describe('global processing transport', () => {
    let release: (() => void)[];
    beforeEach(() => {
        vi.useFakeTimers();
        release = [];
    });
    afterEach(() => {
        release.forEach(cleanup => cleanup());
        vi.useRealTimers();
    });

    it('acknowledges retry counts without clearing failures optimistically or changing pause', async () => {
        const initial = snapshot(1, true);
        initial.workers[0].failed = 2;
        const pending = deferred<{
            snapshot: ProcessingSnapshot;
            assets: number;
            comparisons: number;
        }>();
        const transport = {
            snapshot: async () => initial,
            setPaused: vi.fn(),
            retry: vi.fn(() => pending.promise)
        };
        const client = new ProcessingClient(transport);
        const listener = vi.fn();
        release.push(client.subscribe(listener));
        await vi.advanceTimersByTimeAsync(0);
        const action = client.retry('thumbnails', 'failed');
        await client.retry('thumbnails', 'failed');
        await client.setPaused('thumbnails', false);
        expect(transport.retry).toHaveBeenCalledTimes(1);
        expect(transport.setPaused).not.toHaveBeenCalled();
        expect(listener.mock.lastCall?.[0].snapshot.workers[0].failed).toBe(2);
        pending.resolve({ snapshot: snapshot(2, true), assets: 2, comparisons: 0 });
        await action;
        expect(listener.mock.lastCall?.[0].feedback.thumbnails).toContain(
            '2 files queued for retry'
        );
        expect(listener.mock.lastCall?.[0].snapshot.workers[0].paused).toBe(true);
        expect(listener.mock.lastCall?.[0].pending.size).toBe(0);
    });

    it('preserves failures when retry acknowledgement is unavailable', async () => {
        const initial = snapshot();
        initial.workers[0].failed = 2;
        const client = new ProcessingClient({
            snapshot: async () => initial,
            setPaused: vi.fn(),
            retry: async () => {
                throw new Error('C:/private/log');
            }
        });
        const listener = vi.fn();
        release.push(client.subscribe(listener));
        await vi.advanceTimersByTimeAsync(0);
        await client.retry('thumbnails', 'failed');
        const state = listener.mock.lastCall?.[0] as ProcessingClientState;
        expect(state.snapshot?.workers[0].failed).toBe(2);
        expect(state.error).toContain('could not be confirmed');
        expect(state.error).not.toContain('private');
        expect(state.feedback?.thumbnails).toBe('');
    });

    it('rejects a late retry acknowledgement after refresh observes a new service in the same epoch', async () => {
        const pending = deferred<{
            snapshot: ProcessingSnapshot;
            assets: number;
            comparisons: number;
        }>();
        let current = snapshot();
        const client = new ProcessingClient({
            snapshot: async () => current,
            setPaused: vi.fn(),
            retry: () => pending.promise
        });
        const listener = vi.fn();
        release.push(client.subscribe(listener));
        await vi.advanceTimersByTimeAsync(0);
        const action = client.retry('thumbnails', 'failed');
        current = { ...snapshot(), instance_id: 'restarted' };
        await client.refresh();
        pending.resolve({ snapshot: snapshot(100), assets: 20, comparisons: 0 });
        await action;
        expect(listener.mock.lastCall?.[0].snapshot.instance_id).toBe('restarted');
        expect(listener.mock.lastCall?.[0].feedback?.thumbnails).toBeUndefined();
    });

    it('accepts a restarted service and rejects a late control from the old instance', async () => {
        const old = deferred<ProcessingSnapshot>();
        const restarted = { ...snapshot(1, true), instance_id: 'restarted' };
        const client = new ProcessingClient({
            retry: async () => ({ snapshot: snapshot(), assets: 0, comparisons: 0 }),
            snapshot: async () => snapshot(),
            setPaused: worker =>
                worker === 'thumbnails' ? old.promise : Promise.resolve(restarted)
        });
        const listener = vi.fn();
        release.push(client.subscribe(listener));
        await vi.advanceTimersByTimeAsync(0);
        const previousAction = client.setPaused('thumbnails', true);
        await client.setPaused('colors', true);
        old.resolve(snapshot(100, false));
        await previousAction;
        expect(listener.mock.lastCall?.[0].snapshot.instance_id).toBe('restarted');
        expect(listener.mock.lastCall?.[0].snapshot.workers[0].paused).toBe(true);
    });

    it('shares one polling request across statusbar consumers and releases timers', async () => {
        const transport = {
            retry: async () => ({ snapshot: snapshot(), assets: 0, comparisons: 0 }),
            snapshot: vi.fn(async () => snapshot()),
            setPaused: vi.fn()
        };
        const client = new ProcessingClient(transport);
        release.push(client.subscribe(vi.fn()), client.subscribe(vi.fn()));
        await vi.advanceTimersByTimeAsync(0);
        expect(transport.snapshot).toHaveBeenCalledTimes(1);
        await vi.advanceTimersByTimeAsync(2000);
        expect(transport.snapshot).toHaveBeenCalledTimes(2);
        release.forEach(cleanup => cleanup());
        release = [];
        await vi.advanceTimersByTimeAsync(10000);
        expect(transport.snapshot).toHaveBeenCalledTimes(2);
    });

    it('does not let a poll started before a control undo its acknowledgement', async () => {
        const old = deferred<ProcessingSnapshot>();
        const client = new ProcessingClient({
            retry: async () => ({ snapshot: snapshot(), assets: 0, comparisons: 0 }),
            snapshot: () => old.promise,
            setPaused: async () => snapshot(5, true)
        });
        const listener = vi.fn();
        release.push(client.subscribe(listener));
        await client.setPaused('thumbnails', true);
        old.resolve(snapshot(4, false));
        await vi.advanceTimersByTimeAsync(0);
        expect(listener.mock.lastCall?.[0].snapshot.workers[0].paused).toBe(true);
        expect(listener.mock.lastCall?.[0].snapshot.sequence).toBe(5);
    });

    it('retains known intent after a rejected action and reconciles the actual backend later', async () => {
        let current = snapshot();
        const transport: ProcessingTransport = {
            retry: async () => ({ snapshot: snapshot(), assets: 0, comparisons: 0 }),
            snapshot: async () => current,
            setPaused: async () => {
                throw new Error('IPC rejected');
            }
        };
        const client = new ProcessingClient(transport);
        const listener = vi.fn();
        release.push(client.subscribe(listener));
        await vi.advanceTimersByTimeAsync(0);
        await client.setPaused('thumbnails', true);
        const failed = listener.mock.lastCall?.[0] as ProcessingClientState;
        expect(failed.snapshot?.workers[0].paused).toBe(false);
        expect(failed.error).toBeTruthy();
        expect(failed.pending.size).toBe(0);
        current = snapshot(2, true);
        await vi.advanceTimersByTimeAsync(2000);
        expect(listener.mock.lastCall?.[0].snapshot.workers[0].paused).toBe(true);
        expect(listener.mock.lastCall?.[0].error).toBeNull();
    });

    it('recovers from a hung query and ignores its late result', async () => {
        const hung = deferred<ProcessingSnapshot>();
        const transport = {
            retry: async () => ({ snapshot: snapshot(), assets: 0, comparisons: 0 }),
            snapshot: vi.fn().mockReturnValueOnce(hung.promise).mockResolvedValue(snapshot(8)),
            setPaused: vi.fn()
        };
        const client = new ProcessingClient(transport);
        const listener = vi.fn();
        release.push(client.subscribe(listener));
        await vi.advanceTimersByTimeAsync(10000);
        expect(listener.mock.lastCall?.[0].error).toBeTruthy();
        await vi.advanceTimersByTimeAsync(2000);
        expect(listener.mock.lastCall?.[0].snapshot.sequence).toBe(8);
        hung.resolve(snapshot(1, true));
        await vi.advanceTimersByTimeAsync(0);
        expect(listener.mock.lastCall?.[0].snapshot.sequence).toBe(8);
    });

    it('orders concurrent controls by server sequence instead of arrival time', async () => {
        const first = deferred<ProcessingSnapshot>();
        const second = deferred<ProcessingSnapshot>();
        const client = new ProcessingClient({
            retry: async () => ({ snapshot: snapshot(), assets: 0, comparisons: 0 }),
            snapshot: async () => snapshot(),
            setPaused: worker => (worker === 'thumbnails' ? first.promise : second.promise)
        });
        const listener = vi.fn();
        release.push(client.subscribe(listener));
        await vi.advanceTimersByTimeAsync(0);
        const firstAction = client.setPaused('thumbnails', true);
        const secondAction = client.setPaused('colors', true);
        second.resolve(snapshot(20, true));
        await secondAction;
        first.resolve(snapshot(19, false));
        await firstAction;
        expect(listener.mock.lastCall?.[0].snapshot.sequence).toBe(20);
    });
});
