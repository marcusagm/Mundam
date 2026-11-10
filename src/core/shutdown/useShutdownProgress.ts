import { createMemo, createSignal, onCleanup, onMount } from 'solid-js';
import { acknowledgeShutdownPresentation, observeShutdown } from './client';
import type { ShutdownSnapshot } from './types';

const SLOW_SHUTDOWN_SECONDS = 8;
const UNRESPONSIVE_SECONDS = 5;

/**
 * Adapts the independent shutdown observer into presentation state.
 * @returns Reactive status and elapsed time accessors.
 * @example
 * const shutdown = useShutdownProgress();
 * return <p role="status">{shutdown.message()}</p>;
 */
export function useShutdownProgress() {
    const [snapshot, setSnapshot] = createSignal<ShutdownSnapshot>();
    const [elapsedSeconds, setElapsedSeconds] = createSignal(0);
    const [lastContactSeconds, setLastContactSeconds] = createSignal(0);
    const unavailable = createMemo(
        () => elapsedSeconds() - lastContactSeconds() >= UNRESPONSIVE_SECONDS
    );
    const message = createMemo(() => {
        if (unavailable()) return 'Waiting for the application to respond';
        if (snapshot()?.phase === 'finished' && snapshot()?.degraded) {
            return 'Mundam could not finish closing normally';
        }
        if (snapshot()?.phase === 'draining') return 'Stopping background activity';
        return 'Finishing up. Please wait.';
    });

    onMount(() => {
        const startedAt = performance.now();
        const dispose = observeShutdown(
            state => {
                setSnapshot(state);
                setLastContactSeconds(Math.floor((performance.now() - startedAt) / 1000));
            },
            error => console.warn('Shutdown service unavailable:', error)
        );
        const interval = setInterval(() => {
            setElapsedSeconds(Math.floor((performance.now() - startedAt) / 1000));
        }, 1000);
        let nextFrame = 0;
        const firstFrame = requestAnimationFrame(() => {
            nextFrame = requestAnimationFrame(() => {
                void acknowledgeShutdownPresentation().catch(error =>
                    console.warn('Unable to acknowledge shutdown presentation:', error)
                );
            });
        });
        onCleanup(() => {
            dispose();
            clearInterval(interval);
            cancelAnimationFrame(firstFrame);
            cancelAnimationFrame(nextFrame);
        });
    });

    return {
        /** Whether the backend has not responded recently. */
        unavailable,
        /** Accessible status for the current phase. */
        message,
        /** Actual elapsed presentation time. */
        elapsedSeconds,
        /** Indicates that shutdown is taking longer than usual. */
        isSlow: () => elapsedSeconds() >= SLOW_SHUTDOWN_SECONDS
    };
}
