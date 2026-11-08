import type {
    ProcessingClientState,
    ProcessingSnapshot,
    WorkerKind,
    WorkerSnapshot
} from '../../../../core/store/processing/types';

/** Human-readable executor labels shared by controls and accessible names. */
export const workerNames: Record<WorkerKind, string> = {
    thumbnails: 'Thumbnails and metadata',
    colors: 'Colors',
    duplicates: 'Duplicates'
};

/** Returns drain, lifecycle and automatic-policy states without treating errors as activity. */
export function workerStatus(worker: WorkerSnapshot): string {
    if (!worker.online) return 'Unavailable';
    if (worker.active > 0)
        return worker.paused ? 'Pausing — finishing current operation' : 'Processing';
    if (worker.paused) return 'Paused';
    if (!worker.automatic_enabled) return 'Automatic detection off';
    return workerQueueStatus(worker);
}

function workerQueueStatus(worker: WorkerSnapshot): string {
    if (workerWaiting(worker)) return 'Waiting for processing';
    if (worker.retry_wait > 0) return 'Waiting to retry';
    if ((worker.blocked ?? 0) > 0) return 'Waiting for extraction tools';
    if (worker.failed > 0) return 'Idle with failures';
    return 'Idle';
}

/** Maps backend phases to user-facing activity labels. */
export function phaseLabel(phase: string | null): string {
    const phases: Record<string, string> = {
        thumbnail: 'Generating thumbnails',
        colors: 'Extracting colors',
        fingerprinting: 'Reading fingerprints',
        manual_scan: 'Manual duplicate scan',
        exact_matching: 'Comparing identical files',
        visual_matching: 'Comparing similar images'
    };
    return phase ? phases[phase] || 'Processing' : '';
}

/** True only for executing or eligible automatic work, excluding paused/disabled backlog. */
export function workerBusy(worker: WorkerSnapshot): boolean {
    return (
        worker.active > 0 ||
        (worker.online && !worker.paused && worker.automatic_enabled && workerWaiting(worker))
    );
}

function workerWaiting(worker: WorkerSnapshot): boolean {
    return worker.pending > 0 || worker.scheduled;
}

/** Aggregate display states preserve uncertainty and distinguish retries from active work. */
export type ProcessingIndicator = 'unknown' | 'busy' | 'paused' | 'warning' | 'idle';

function manualScanBusy(scan: ProcessingSnapshot['manual_scan']): boolean {
    return scan.is_running && !['paused', 'retry_wait'].includes(scan.phase);
}

/** Missing snapshots and query failures never imply healthy idle workers. */
export function processingIndicator(
    state: ProcessingClientState,
    indexing: boolean
): ProcessingIndicator {
    if (state.error || !state.snapshot) return 'unknown';
    if (
        indexing ||
        state.snapshot.workers.some(workerBusy) ||
        manualScanBusy(state.snapshot.manual_scan)
    )
        return 'busy';
    if (
        state.snapshot.matching?.failed ||
        state.snapshot.matching?.retry_wait ||
        state.snapshot.workers.some(
            worker =>
                !worker.online ||
                worker.failed > 0 ||
                worker.retry_wait > 0 ||
                (worker.blocked ?? 0) > 0
        )
    )
        return 'warning';
    if (state.snapshot.workers.some(worker => worker.paused)) return 'paused';
    return 'idle';
}

/** Accessible labels also drive the statusbar's central message. */
export const processingLabels: Record<ProcessingIndicator, string> = {
    unknown: 'Processing status unavailable',
    busy: 'Processing library',
    paused: 'Processing paused',
    warning: 'Processing needs attention',
    idle: 'Ready'
};

/** Returns semantic badge variant according to worker health and activity. */
export function workerStatusVariant(
    worker: WorkerSnapshot
): 'default' | 'outline' | 'secondary' | 'success' | 'warning' | 'error' | 'info' {
    if (!worker.online) return 'secondary';
    if (worker.paused) return 'secondary';
    if (worker.active > 0) return 'info';
    if (worker.failed > 0 || (worker.blocked ?? 0) > 0) return 'warning';
    if (worker.retry_wait > 0) return 'warning';
    if (workerWaiting(worker)) return 'outline';
    return 'success';
}

/** Human-friendly copy for worker status and activity. */
export function humanWorkerPhaseLabel(phase: string | null, workerKind: WorkerKind): string {
    if (!phase) {
        if (workerKind === 'thumbnails') return 'Generates high-resolution previews';
        if (workerKind === 'colors') return 'Extracts primary and palette colors';
        if (workerKind === 'duplicates') return 'Compares image similarities';
        return 'Processing queue';
    }
    const humanPhases: Record<string, string> = {
        thumbnail: 'Generating previews',
        colors: 'Extracting color palette',
        fingerprinting: 'Analyzing visual similarity',
        manual_scan: 'Scanning for duplicate media',
        exact_matching: 'Comparing identical files',
        visual_matching: 'Comparing similar images'
    };
    return humanPhases[phase] || phaseLabel(phase);
}

/** Computes aggregated items count across all active workers. */
export function calculateTotalBacklog(workers: WorkerSnapshot[]): {
    totalActive: number;
    totalPending: number;
    totalFailed: number;
} {
    let totalActive = 0;
    let totalPending = 0;
    let totalFailed = 0;

    for (const worker of workers) {
        totalActive += worker.active;
        totalPending += worker.pending;
        totalFailed += worker.failed;
    }

    return { totalActive, totalPending, totalFailed };
}
