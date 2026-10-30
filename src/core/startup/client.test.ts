// @vitest-environment node
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { StartupSnapshot } from './types';
import { observeStartup, waitForBackend } from './client';

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: mocks.listen }));

/** Builds a retained state with controlled revision for race-condition tests. */
function snapshot(revision: number, backendReady = false): StartupSnapshot {
    return {
        revision,
        stage: backendReady ? 'backendReady' : 'database',
        backendReady,
        failed: false,
        errorCode: null
    };
}

describe('startup observation', () => {
    beforeEach(() => {
        vi.useFakeTimers();
        vi.resetAllMocks();
        mocks.listen.mockResolvedValue(vi.fn());
    });
    afterEach(() => vi.useRealTimers());

    it('does not regress when an older query resolves after a newer event', async () => {
        let resolveQuery!: (value: StartupSnapshot) => void;
        mocks.invoke.mockReturnValue(
            new Promise<StartupSnapshot>(resolve => {
                resolveQuery = resolve;
            })
        );
        const receive = vi.fn();
        const dispose = observeStartup(receive, vi.fn());
        await vi.advanceTimersByTimeAsync(0);
        const listener = mocks.listen.mock.calls[0][1];
        listener({ payload: snapshot(4, true) });
        resolveQuery(snapshot(2));
        await vi.advanceTimersByTimeAsync(0);
        expect(receive).toHaveBeenCalledTimes(1);
        expect(receive).toHaveBeenLastCalledWith(snapshot(4, true));
        dispose();
    });

    it('recovers missed readiness events by querying retained state', async () => {
        const unlisten = vi.fn();
        mocks.listen.mockResolvedValue(unlisten);
        mocks.invoke.mockResolvedValueOnce(snapshot(1)).mockResolvedValue(snapshot(5, true));
        const ready = waitForBackend();
        await vi.advanceTimersByTimeAsync(1000);
        await expect(ready).resolves.toBeUndefined();
        expect(unlisten).toHaveBeenCalledOnce();
        expect(vi.getTimerCount()).toBe(0);
    });

    it('retains backend failure instead of resolving readiness', async () => {
        mocks.invoke.mockResolvedValue({
            ...snapshot(3),
            failed: true,
            errorCode: 'BACKEND_STARTUP_FAILED'
        });
        await expect(waitForBackend()).rejects.toThrow('BACKEND_STARTUP_FAILED');
        expect(vi.getTimerCount()).toBe(0);
    });

    it('releases a listener registered after its observer was disposed', async () => {
        let resolveListener!: (dispose: () => void) => void;
        mocks.listen.mockReturnValue(
            new Promise<() => void>(resolve => {
                resolveListener = resolve;
            })
        );
        const unlisten = vi.fn();
        const dispose = observeStartup(vi.fn(), vi.fn());
        dispose();
        resolveListener(unlisten);
        await vi.advanceTimersByTimeAsync(0);
        expect(unlisten).toHaveBeenCalledOnce();
        expect(mocks.invoke).not.toHaveBeenCalled();
    });
});
