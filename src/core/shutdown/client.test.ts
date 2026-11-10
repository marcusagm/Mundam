// @vitest-environment node
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { observeShutdown } from './client';
import type { ShutdownSnapshot } from './types';

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: mocks.listen }));

function snapshot(revision: number): ShutdownSnapshot {
    return { revision, phase: 'draining', degraded: false };
}

describe('shutdown observation', () => {
    beforeEach(() => {
        vi.useFakeTimers();
        vi.resetAllMocks();
        mocks.listen.mockResolvedValue(vi.fn());
    });
    afterEach(() => vi.useRealTimers());

    it('rejects stale query responses after newer events', async () => {
        let resolveQuery!: (value: ShutdownSnapshot) => void;
        mocks.invoke.mockReturnValue(
            new Promise<ShutdownSnapshot>(resolve => (resolveQuery = resolve))
        );
        const receive = vi.fn();
        const dispose = observeShutdown(receive, vi.fn());
        await vi.advanceTimersByTimeAsync(0);
        mocks.listen.mock.calls[0][1]({ payload: snapshot(4) });
        resolveQuery(snapshot(2));
        await vi.advanceTimersByTimeAsync(0);
        expect(receive).toHaveBeenCalledOnce();
        expect(receive).toHaveBeenCalledWith(snapshot(4));
        dispose();
    });

    it('recovers missed events and confirms contact at the same revision', async () => {
        mocks.invoke.mockResolvedValue(snapshot(2));
        const receive = vi.fn();
        const dispose = observeShutdown(receive, vi.fn());
        await vi.advanceTimersByTimeAsync(1000);
        expect(receive).toHaveBeenCalledTimes(2);
        dispose();
        expect(vi.getTimerCount()).toBe(0);
    });

    it('disposes a listener that registers after cleanup', async () => {
        let resolveListener!: (dispose: () => void) => void;
        mocks.listen.mockReturnValue(
            new Promise<() => void>(resolve => {
                resolveListener = resolve;
            })
        );
        const unlisten = vi.fn();
        const dispose = observeShutdown(vi.fn(), vi.fn());
        dispose();
        resolveListener(unlisten);
        await vi.advanceTimersByTimeAsync(0);
        expect(unlisten).toHaveBeenCalledOnce();
        expect(mocks.invoke).not.toHaveBeenCalled();
    });
});
