import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { ShutdownSnapshot } from './types';

const SNAPSHOT_INTERVAL_MILLISECONDS = 1000;

/**
 * Observes shutdown events and reconciles them against retained state.
 * @param onSnapshot - Receives non-stale state and contact confirmation.
 * @param onUnavailable - Reports a failed state query.
 * @returns A disposer for the listener and polling interval.
 * @example
 * const dispose = observeShutdown(console.info, console.warn);
 * dispose();
 */
export function observeShutdown(
    onSnapshot: (snapshot: ShutdownSnapshot) => void,
    onUnavailable: (error: unknown) => void
): () => void {
    let disposed = false;
    let revision = -1;
    let unlisten: (() => void) | undefined;
    let queryPending = false;
    const accept = (snapshot: ShutdownSnapshot): void => {
        if (disposed || snapshot.revision < revision) return;
        revision = snapshot.revision;
        onSnapshot(snapshot);
    };
    const query = async (): Promise<void> => {
        if (disposed || queryPending) return;
        queryPending = true;
        try {
            accept(await invoke<ShutdownSnapshot>('get_shutdown_state'));
        } catch (error) {
            if (!disposed) onUnavailable(error);
        } finally {
            queryPending = false;
        }
    };
    void listen<ShutdownSnapshot>('shutdown:changed', event => accept(event.payload))
        .then(dispose => {
            if (disposed) dispose();
            else unlisten = dispose;
        })
        .catch(error => {
            if (!disposed)
                console.warn('Shutdown events unavailable; using retained state:', error);
        })
        .finally(() => void query());
    const interval = setInterval(() => void query(), SNAPSHOT_INTERVAL_MILLISECONDS);
    return (): void => {
        disposed = true;
        clearInterval(interval);
        unlisten?.();
    };
}

/**
 * Reports that the dedicated shutdown window had an opportunity to render.
 * @returns A promise that rejects if the native acknowledgement fails.
 * @example
 * await acknowledgeShutdownPresentation();
 */
export function acknowledgeShutdownPresentation(): Promise<void> {
    return invoke('acknowledge_shutdown_presentation');
}
