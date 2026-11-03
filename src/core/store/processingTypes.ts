/** Executor identities accepted by the desktop control API. */
export type WorkerKind = 'thumbnails' | 'colors' | 'duplicates';

/** Runtime activity and durable queue counts are independent of AssetState. */
export interface WorkerSnapshot {
    /** Stable executor identity. */
    worker: WorkerKind;
    /** Whether the orchestration task is alive. */
    online: boolean;
    /** Session intent; active operations drain before becoming fully paused. */
    paused: boolean;
    /** Currently admitted operations. */
    active: number;
    /** Current algorithm phase, without source paths. */
    phase: string | null;
    /** Follow-up comparison scheduled between fingerprint batches and matching. */
    scheduled: boolean;
    /** Assets eligible for admission now. */
    pending: number;
    /** Assets awaiting bounded retry backoff. */
    retry_wait: number;
    /** Exhausted attempts for the current source generation. */
    failed: number;
    /** Unsupported operations for the current source generation. */
    skipped: number;
    /** Automatic duplicate detection can be disabled separately from pause. */
    automatic_enabled: boolean;
}

/** Full replacement read model; no library invalidation is implied. */
export interface ProcessingSnapshot {
    /** Application service identity, changed after restart. */
    instance_id: string;
    /** Monotonic response sequence within this service instance. */
    sequence: number;
    /** Stable worker rows. */
    workers: WorkerSnapshot[];
    /** Progress from the existing manually requested duplicate scan. */
    manual_scan: { is_running: boolean; phase: string; processed: number; total: number };
}

/** Shared statusbar transport state. */
export interface ProcessingClientState {
    /** Last authoritative snapshot, retained during temporary transport failures. */
    snapshot: ProcessingSnapshot | null;
    /** Staleness or explicit action failure, displayed inline without toasts. */
    error: string | null;
    /** Controls awaiting backend acknowledgement. */
    pending: ReadonlySet<WorkerKind>;
}
