import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { StartupSnapshot, StartupStage } from './types';

const SNAPSHOT_INTERVAL_MILLISECONDS = 1000;

/**
 * Reconciles retained state and events by revision, recovering missed events.
 * @param onSnapshot - Receives monotonic state updates.
 * @param onUnavailable - Reports inability to contact the service.
 * @returns A disposer that also releases listeners registered after disposal.
 * @example
 * const dispose = observeStartup(setSnapshot, console.error);
 * onCleanup(dispose);
 */
export function observeStartup(
    onSnapshot: (snapshot: StartupSnapshot) => void,
    onUnavailable: (error: unknown) => void
): () => void {
    let disposed = false;
    let revision = -1;
    let unlisten: (() => void) | undefined;
    let queryPending = false;
    const accept = (snapshot: StartupSnapshot): void => {
        if (disposed || snapshot.revision < revision) return;
        revision = snapshot.revision;
        onSnapshot(snapshot);
    };
    const query = async (): Promise<void> => {
        if (disposed || queryPending) return;
        queryPending = true;
        try {
            accept(await invoke<StartupSnapshot>('get_startup_state'));
        } catch (error) {
            if (!disposed) onUnavailable(error);
        } finally {
            queryPending = false;
        }
    };
    void listen<StartupSnapshot>('startup:changed', event => accept(event.payload))
        .then(dispose => {
            if (disposed) dispose();
            else unlisten = dispose;
        })
        .catch(error => {
            if (!disposed) console.warn('Startup events unavailable; using retained state:', error);
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
 * Gates library-dependent IPC on actual backend readiness.
 * @returns Resolves when essential backend services exist.
 * @throws When startup fails or the service cannot be reached.
 * @example
 * await waitForBackend();
 * await systemActions.initialize();
 */
export function waitForBackend(): Promise<void> {
    return new Promise((resolve, reject) => {
        const dispose = observeStartup(
            snapshot => {
                if (snapshot.failed) {
                    dispose();
                    reject(new Error(snapshot.errorCode ?? 'BACKEND_STARTUP_FAILED'));
                } else if (snapshot.backendReady) {
                    dispose();
                    resolve();
                }
            },
            error => {
                dispose();
                reject(error);
            }
        );
    });
}

/**
 * Reports a frontend milestone to the startup adapter.
 * @param stage - Milestone reached after backend readiness.
 * @returns Resolves after the state is retained.
 * @example
 * await reportStartupStage('appearance');
 */
export function reportStartupStage(stage: StartupStage): Promise<void> {
    return invoke('report_startup_stage', { stage });
}
