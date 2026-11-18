/** Development-only fault injection for native shutdown validation. */
export type ShutdownTestFault = 'ipc-unavailable' | 'presentation-unavailable';

/**
 * Enables an explicitly requested fault only in the Vite development server.
 * @param fault - Failure being exercised by a manual desktop test.
 * @returns Whether this development run requested the failure.
 */
export function hasShutdownTestFault(fault: ShutdownTestFault): boolean {
    return import.meta.env.DEV && import.meta.env.VITE_SHUTDOWN_TEST_FAULT === fault;
}
