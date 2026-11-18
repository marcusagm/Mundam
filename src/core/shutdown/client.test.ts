// @vitest-environment node
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { acknowledgeShutdownPresentation, observeShutdown } from './client';
import { hasShutdownTestFault } from './testFaults';
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
    afterEach(() => {
        vi.useRealTimers();
        vi.unstubAllEnvs();
    });

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

describe('shutdown IPC recovery', () => {
    beforeEach(() => {
        vi.useFakeTimers();
        vi.resetAllMocks();
        mocks.listen.mockResolvedValue(vi.fn());
    });
    afterEach(() => {
        vi.useRealTimers();
        vi.unstubAllEnvs();
    });

    it('recovers after a state query never responds', async () => {
        mocks.invoke.mockImplementationOnce(() => new Promise(() => {}));
        mocks.invoke.mockResolvedValue(snapshot(3));
        const receive = vi.fn();
        const unavailable = vi.fn();
        const dispose = observeShutdown(receive, unavailable);
        await vi.advanceTimersByTimeAsync(2100);
        expect(unavailable).toHaveBeenCalledOnce();
        expect(receive).toHaveBeenCalledWith(snapshot(3));
        dispose();
        expect(vi.getTimerCount()).toBe(0);
    });

    it('retries a transient acknowledgement failure', async () => {
        mocks.invoke.mockRejectedValueOnce(new Error('temporary IPC error'));
        mocks.invoke.mockResolvedValueOnce(undefined);
        const acknowledgement = acknowledgeShutdownPresentation();
        await vi.advanceTimersByTimeAsync(100);
        await acknowledgement;
        expect(mocks.invoke).toHaveBeenCalledTimes(2);
        expect(vi.getTimerCount()).toBe(0);
    });

    it('bounds acknowledgement retries when IPC never responds', async () => {
        mocks.invoke.mockImplementation(() => new Promise(() => {}));
        const result = acknowledgeShutdownPresentation().catch(error => error);
        await vi.advanceTimersByTimeAsync(2100);
        expect(await result).toBeInstanceOf(Error);
        expect(mocks.invoke).toHaveBeenCalledTimes(2);
        expect(vi.getTimerCount()).toBe(0);
    });

    it('keeps development faults inactive in production', () => {
        vi.stubEnv('DEV', false);
        vi.stubEnv('VITE_SHUTDOWN_TEST_FAULT', 'ipc-unavailable');
        expect(hasShutdownTestFault('ipc-unavailable')).toBe(false);
    });

    it('injects IPC failure without calling native commands', async () => {
        vi.stubEnv('DEV', true);
        vi.stubEnv('VITE_SHUTDOWN_TEST_FAULT', 'ipc-unavailable');
        const result = acknowledgeShutdownPresentation().catch(error => error);
        await vi.advanceTimersByTimeAsync(100);
        expect(await result).toBeInstanceOf(Error);
        expect(mocks.invoke).not.toHaveBeenCalled();
    });
});
