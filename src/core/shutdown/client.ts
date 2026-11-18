import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { ShutdownSnapshot } from './types';
import { hasShutdownTestFault } from './testFaults';

const SNAPSHOT_INTERVAL_MILLISECONDS = 1000;
const IPC_TIMEOUT_MILLISECONDS = 1000;
const ACKNOWLEDGEMENT_RETRY_MILLISECONDS = 100;

/**
 * Bounds frontend IPC waits without coupling backend cleanup to a response.
 * @param command - Restricted shutdown command to invoke.
 * @returns The response or a timeout/failure; late native responses are ignored.
 */
async function invokeShutdown<T>(command: string): Promise<T> {
    if (hasShutdownTestFault('ipc-unavailable')) {
        throw new Error('Injected shutdown IPC failure');
    }
    let timer: ReturnType<typeof setTimeout> | undefined;
    try {
        return await Promise.race([
            invoke<T>(command),
            new Promise<never>((_, reject) => {
                timer = setTimeout(
                    () => reject(new Error('Shutdown IPC response timed out')),
                    IPC_TIMEOUT_MILLISECONDS
                );
            })
        ]);
    } finally {
        clearTimeout(timer);
    }
}

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
            accept(await invokeShutdown<ShutdownSnapshot>('get_shutdown_state'));
        } catch (error) {
            if (!disposed) onUnavailable(error);
        } finally {
            queryPending = false;
        }
    };
    const subscription = hasShutdownTestFault('ipc-unavailable')
        ? Promise.reject(new Error('Injected shutdown event failure'))
        : listen<ShutdownSnapshot>('shutdown:changed', event => accept(event.payload));
    void subscription
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
export async function acknowledgeShutdownPresentation(): Promise<void> {
    try {
        await invokeShutdown<void>('acknowledge_shutdown_presentation');
    } catch {
        // Retrying is safe because the backend acknowledgement is idempotent.
        await new Promise(resolve => setTimeout(resolve, ACKNOWLEDGEMENT_RETRY_MILLISECONDS));
        await invokeShutdown<void>('acknowledge_shutdown_presentation');
    }
}
