import type { ProcessingClientState, ProcessingSnapshot, WorkerKind } from './processingTypes';

const POLL_INTERVAL = 2000;
const REQUEST_TIMEOUT = 10000;

/** Injectable IPC boundary used by transport tests. */
export interface ProcessingTransport {
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

    /** @param transport - Desktop commands, injectable without Solid or a webview. */
    constructor(private transport: ProcessingTransport) {}

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

    private accept(snapshot: ProcessingSnapshot, epoch: number): void {
        const previous = this.state.snapshot;
        if (
            previous?.instance_id === snapshot.instance_id &&
            previous.sequence >= snapshot.sequence
        )
            return;
        if (previous && previous.instance_id !== snapshot.instance_id && epoch !== this.epoch)
            return;
        this.publish({ ...this.state, snapshot, error: null });
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
