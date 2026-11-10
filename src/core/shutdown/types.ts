/** Retained application shutdown state exposed by the native coordinator. */
export interface ShutdownSnapshot {
    /** Monotonic revision used to discard stale responses. */
    revision: number;
    /** Current phase of the registered task shutdown. */
    phase: 'idle' | 'requested' | 'draining' | 'finished';
    /** Whether a registered task failed or exceeded its timeout. */
    degraded: boolean;
}
