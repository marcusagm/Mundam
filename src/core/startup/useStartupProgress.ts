import { createMemo, createSignal, onCleanup, onMount } from 'solid-js';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { observeStartup } from './client';
import { STARTUP_MESSAGES, type StartupSnapshot } from './types';

const SLOW_STARTUP_SECONDS = 8;
const UNRESPONSIVE_SECONDS = 5;

/**
 * Adapts startup observations into accessible presentation state.
 * @returns Status accessors and the native close action.
 * @example
 * const startup = useStartupProgress();
 * return <p role="status">{startup.message()}</p>;
 */
export function useStartupProgress() {
    const [snapshot, setSnapshot] = createSignal<StartupSnapshot>();
    const [elapsedSeconds, setElapsedSeconds] = createSignal(0);
    const [lastContactSeconds, setLastContactSeconds] = createSignal(0);
    const [closeError, setCloseError] = createSignal(false);
    const failed = createMemo(() => snapshot()?.failed === true);
    const unavailable = createMemo(
        () => elapsedSeconds() - lastContactSeconds() >= UNRESPONSIVE_SECONDS
    );
    const message = createMemo(() => {
        if (failed()) return 'Mundam could not start';
        if (unavailable()) return 'Waiting for the application to respond';
        return STARTUP_MESSAGES[snapshot()?.stage ?? 'preparing'];
    });

    onMount(() => {
        const startedAt = performance.now();
        const dispose = observeStartup(
            state => {
                setSnapshot(state);
                setLastContactSeconds(Math.floor((performance.now() - startedAt) / 1000));
            },
            error => console.warn('Startup service unavailable:', error)
        );
        const interval = setInterval(() => {
            setElapsedSeconds(Math.floor((performance.now() - startedAt) / 1000));
        }, 1000);
        onCleanup(() => {
            dispose();
            clearInterval(interval);
        });
    });

    /** Closes startup through the native graceful-shutdown path. */
    const close = async (): Promise<void> => {
        try {
            await getCurrentWindow().close();
        } catch (error) {
            console.error('Unable to close startup window:', error);
            setCloseError(true);
        }
    };

    return {
        /** Retained backend state for diagnostic codes. */
        snapshot,
        /** Actual wall-clock startup duration. */
        elapsedSeconds,
        /** Terminal initialization failure. */
        failed,
        /** Indicates lost IPC responses, independent from visual animation. */
        unavailable,
        /** Current accessible stage description. */
        message,
        /** Reveals contextual information without changing readiness. */
        isSlow: () => elapsedSeconds() >= SLOW_STARTUP_SECONDS,
        /** Indicates native shutdown could not be requested. */
        closeError,
        /** Requests graceful shutdown. */
        close
    };
}
