import type {
    ProcessingClientState,
    ProcessingSnapshot,
    WorkerKind,
    ProcessingRetryScope,
    ProcessingRetryAcknowledgement
} from './types';
import { formatProcessingRetry } from '../../../utils/processing';

const POLL_INTERVAL = 2000;
const REQUEST_TIMEOUT = 10000;
const MAX_RETIRED_INSTANCES = 8;

/** Injectable IPC boundary used by transport tests. */
export interface ProcessingTransport {
    /** Queues only eligible terminal work and returns its authoritative snapshot. */
    retry: (
        worker: WorkerKind,
        scope: ProcessingRetryScope
    ) => Promise<ProcessingRetryAcknowledgement>;
    /** Reads a full snapshot, even after missed notifications. */
    snapshot: () => Promise<ProcessingSnapshot>;
    /** Returns the backend acknowledgement of a new pause intent. */
    setPaused: (worker: WorkerKind, paused: boolean) => Promise<ProcessingSnapshot>;
}

/** Limits hangs while leaving late RPC completions unable to mutate client state. */
async function withTimeout<Result>(request: Promise<Result>): Promise<Result> {
    let timer: ReturnType<typeof setTimeout> | undefined;
    try {
        return await Promise.race([
            request,
            new Promise<never>((_resolve, reject) => {
                timer = setTimeout(
                    () => reject(new Error('Processing request timed out')),
                    REQUEST_TIMEOUT
                );
            })
        ]);
    } finally {
        clearTimeout(timer);
    }
}

/** Polls one global read model, fences responses and never changes asset stores. */
export class ProcessingClient {
    private state: ProcessingClientState = { snapshot: null, error: null, pending: new Set() };
    private listeners = new Set<(state: ProcessingClientState) => void>();
    private timer: ReturnType<typeof setTimeout> | undefined;
    private polling = false;
    private epoch = 0;
    private retiredInstances = new Set<string>();

    /** @param transport - Desktop commands, injectable without Solid or a webview. */
    constructor(private transport: ProcessingTransport) {}

    /** Reconciles explicit actions through the same fenced transport as polling. */
    refresh(): Promise<void> {
        return this.poll();
    }

    /** Subscribes the statusbar and releases polling when its final consumer unmounts. */
    subscribe(listener: (state: ProcessingClientState) => void): () => void {
        this.listeners.add(listener);
        listener(this.state);
        if (this.listeners.size === 1) void this.poll();
        return () => {
            this.listeners.delete(listener);
            if (this.listeners.size === 0) {
                clearTimeout(this.timer);
                this.epoch++;
            }
        };
    }

    /** Changes UI intent only after acknowledgement; errors retain the actual known state. */
    async setPaused(worker: WorkerKind, paused: boolean): Promise<void> {
        if (this.state.pending.has(worker)) return;
        const epoch = ++this.epoch;
        const pending = new Set(this.state.pending);
        pending.add(worker);
        this.publish({ ...this.state, pending, error: null });
        try {
            this.accept(await withTimeout(this.transport.setPaused(worker, paused)), epoch);
        } catch {
            this.publish({
                ...this.state,
                error: 'The processing control could not be confirmed. Status will be refreshed.'
            });
        } finally {
            const pending = new Set(this.state.pending);
            pending.delete(worker);
            this.publish({ ...this.state, pending });
            this.schedule();
        }
    }

    private publish(state: ProcessingClientState): void {
        this.state = state;
        this.listeners.forEach(listener => listener(state));
    }

    /** Retries are idempotent queue requests; never optimistically clear failures or announce success. */
    async retry(worker: WorkerKind, scope: ProcessingRetryScope): Promise<void> {
        if (this.state.pending.has(worker)) return;
        const epoch = ++this.epoch;
        this.publish({
            ...this.state,
            pending: new Set([...this.state.pending, worker]),
            error: null,
            feedback: { ...this.state.feedback, [worker]: '' }
        });
        try {
            const acknowledgement = await withTimeout(this.transport.retry(worker, scope));
            this.accept(acknowledgement.snapshot, epoch);
            if (this.state.snapshot?.instance_id === acknowledgement.snapshot.instance_id) {
                this.publish({
                    ...this.state,
                    feedback: {
                        ...this.state.feedback,
                        [worker]: formatProcessingRetry(acknowledgement, scope)
                    }
                });
            }
        } catch {
            this.publish({
                ...this.state,
                error: 'The retry request could not be confirmed. Status will be refreshed.'
            });
        } finally {
            const pending = new Set(this.state.pending);
            pending.delete(worker);
            this.publish({ ...this.state, pending });
            this.schedule();
        }
    }

    private accept(snapshot: ProcessingSnapshot, epoch: number): void {
        if (this.retiredInstances.has(snapshot.instance_id)) return;
        const previous = this.state.snapshot;
        if (
            previous?.instance_id === snapshot.instance_id &&
            previous.sequence >= snapshot.sequence
        )
            return;
        if (previous && previous.instance_id !== snapshot.instance_id && epoch !== this.epoch)
            return;
        if (previous && previous.instance_id !== snapshot.instance_id) {
            this.retireInstance(previous.instance_id);
        }
        this.publish({ ...this.state, snapshot, error: null });
    }

    private retireInstance(instance: string): void {
        this.retiredInstances.add(instance);
        if (this.retiredInstances.size > MAX_RETIRED_INSTANCES) {
            const oldest = this.retiredInstances.values().next().value;
            if (oldest) this.retiredInstances.delete(oldest);
        }
        this.state = { ...this.state, feedback: {} };
    }

    private schedule(): void {
        clearTimeout(this.timer);
        if (this.listeners.size > 0) this.timer = setTimeout(() => void this.poll(), POLL_INTERVAL);
    }

    private async poll(): Promise<void> {
        if (this.polling) return;
        this.polling = true;
        const epoch = this.epoch;
        try {
            const snapshot = await withTimeout(this.transport.snapshot());
            if (epoch === this.epoch) this.accept(snapshot, epoch);
        } catch {
            if (epoch === this.epoch) {
                this.publish({
                    ...this.state,
                    error: 'Processing status is unavailable. Displayed information may be out of date.'
                });
            }
        } finally {
            this.polling = false;
            this.schedule();
        }
    }
}
