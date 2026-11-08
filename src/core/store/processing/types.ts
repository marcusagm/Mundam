/** Executor identities accepted by the desktop control API. */
export type WorkerKind = 'thumbnails' | 'colors' | 'duplicates';

/** Separate intents prevent unsupported files from being retried automatically. */
export type ProcessingRetryScope = 'failed' | 'unsupported';

/** Safe grouped causes; raw logs and paths never cross this contract. */
export interface ProcessingDiagnostic {
    issue: string;
    state: 'failed' | 'retry_wait' | 'skipped' | 'blocked';
    count: number;
}

/** Queue acknowledgement is not a processing completion notification. */
export interface ProcessingRetryAcknowledgement {
    snapshot: ProcessingSnapshot;
    assets: number;
    comparisons: number;
}

/** Runtime activity and durable queue counts are independent of AssetState. */
export interface WorkerSnapshot {
    /** Optional for older running backends until the application restarts. */
    diagnostics?: ProcessingDiagnostic[];
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
    /** Suspended until the required extraction tool becomes available. */
    blocked?: number;
    /** Exhausted attempts for the current source generation. */
    failed: number;
    /** Unsupported operations for the current source generation. */
    skipped: number;
    /** Automatic duplicate detection can be disabled separately from pause. */
    automatic_enabled: boolean;
}

/** Operation limits, distinct from native extractor thread counts. */
export interface ProcessingCapacity {
    limit: number;
    active: number;
    waiting: number;
    thumbnail_limit: number;
}

/** Full replacement read model; no library invalidation is implied. */
export interface ProcessingSnapshot {
    /** Optional tools do not imply that all assets require them. */
    tools?: ExtractionToolStatus[];
    /** Operation limits, distinct from native extractor thread counts. */
    capacity?: ProcessingCapacity;
    /** Persisted comparison outcomes, separate from the per-asset queue. */
    matching?: {
        publication_sequence: number;
        pending: number;
        retry_wait: number;
        failed: number;
        completed: string[];
        failed_phases: string[];
        manual_pending: boolean;
        manual_cancel_requested: boolean;
        manual_phase: string | null;
    };
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
    /** Bounded capability check awaiting acknowledgement. */
    checkingTools?: boolean;
    /** Most recent explicit action acknowledgement per worker. */
    feedback?: Partial<Record<WorkerKind, string>>;
    /** Last authoritative snapshot, retained during temporary transport failures. */
    snapshot: ProcessingSnapshot | null;
    /** Staleness or explicit action failure, displayed inline without toasts. */
    error: string | null;
    /** Controls awaiting backend acknowledgement. */
    pending: ReadonlySet<WorkerKind>;
}

/** Safe inventory without executable paths or process output. */
export interface ExtractionToolStatus {
    tool: 'ffmpeg' | 'ffprobe' | 'assimp';
    state: 'unknown' | 'available' | 'unavailable';
    checked_at: number | null;
}
