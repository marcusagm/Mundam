import { invoke } from '@tauri-apps/api/core';

/** Processing outcomes exposed by the bounded thumbnail read model. */
export type ThumbnailProcessingState =
    | 'pending'
    | 'retry_wait'
    | 'failed'
    | 'skipped'
    | 'succeeded'
    | 'unavailable'
    | 'unknown';

/** A compact snapshot; raw backend errors and source paths are not exposed. */
export interface ThumbnailProcessingSnapshot {
    /** Asset represented by this outcome. */
    asset_id: string;
    /** Authoritative processing outcome, or unknown after a transport failure. */
    state: ThumbnailProcessingState;
    /** Regeneration generation used for image cache invalidation. */
    generation: number;
    /** Output recovered even if thumbnail:ready was missed. */
    thumbnail_path: string | null;
}

type Listener = (snapshot: ThumbnailProcessingSnapshot | null) => void;
type Loader = (identifiers: string[]) => Promise<ThumbnailProcessingSnapshot[]>;

interface Subscription {
    listeners: Set<Listener>;
    snapshot: ThumbnailProcessingSnapshot | null;
    epoch: number;
    checkedAt: number;
    dirty: boolean;
    refreshFailed: boolean;
}

const BATCH_SIZE = 200;
const BATCH_DELAY = 30;
const POLL_INTERVAL = 2000;

/** Coalesces visible assets, polls only unresolved work and releases unmounted subscriptions. */
export class ThumbnailProcessingClient {
    private entries = new Map<string, Subscription>();
    private timer: ReturnType<typeof setTimeout> | undefined;
    private inFlight = false;
    private outageReported = false;

    /** @param load - Bounded backend status query, injectable for transport tests. */
    constructor(private readonly load: Loader) {}

    /**
     * Registers interest without issuing an RPC for each thumbnail component.
     * @param identifier - Visible asset ID.
     * @param listener - Receives only this asset's snapshots.
     * @returns A cleanup that releases the entry when its last component unmounts.
     */
    subscribe(identifier: string, listener: Listener): () => void {
        let entry = this.entries.get(identifier);
        if (!entry) {
            entry = {
                listeners: new Set(),
                snapshot: null,
                epoch: 0,
                checkedAt: 0,
                dirty: true,
                refreshFailed: false
            };
            this.entries.set(identifier, entry);
        }
        entry.listeners.add(listener);
        listener(entry.snapshot);
        this.scheduleNext();
        return () => {
            entry.listeners.delete(listener);
            if (entry.listeners.size === 0 && this.entries.get(identifier) === entry) {
                this.entries.delete(identifier);
            }
            this.scheduleNext();
        };
    }

    /**
     * Invalidates snapshots after an explicit request or a ready event.
     * @param identifier - Asset whose generation may have changed.
     * @returns Nothing; mounted consumers receive a fresh batched snapshot.
     */
    invalidate(identifier: string): void {
        const entry = this.entries.get(identifier);
        if (!entry) return;
        entry.epoch++;
        entry.dirty = true;
        this.scheduleNext();
    }

    private isPolling(entry: Subscription): boolean {
        return (
            entry.refreshFailed ||
            ['pending', 'retry_wait', 'unknown'].includes(entry.snapshot?.state || 'pending')
        );
    }

    private isDue(entry: Subscription): boolean {
        return (
            entry.dirty || (this.isPolling(entry) && Date.now() - entry.checkedAt >= POLL_INTERVAL)
        );
    }

    private scheduleNext(): void {
        clearTimeout(this.timer);
        this.timer = undefined;
        if (this.inFlight || this.entries.size === 0) return;
        const entries = [...this.entries.values()];
        let delay = BATCH_DELAY;
        if (!entries.some(entry => this.isDue(entry))) {
            const polling = entries.filter(entry => this.isPolling(entry));
            if (polling.length === 0) return;
            delay = Math.max(
                BATCH_DELAY,
                Math.min(...polling.map(entry => POLL_INTERVAL - (Date.now() - entry.checkedAt)))
            );
        }
        this.timer = setTimeout(() => void this.flush(), delay);
    }

    private publish(
        identifier: string,
        entry: Subscription,
        epoch: number,
        snapshot: ThumbnailProcessingSnapshot,
        refreshFailed = false
    ): void {
        if (this.entries.get(identifier) !== entry || entry.epoch !== epoch) return;
        entry.snapshot = snapshot;
        entry.refreshFailed = refreshFailed;
        entry.checkedAt = Date.now();
        entry.listeners.forEach(listener => listener(snapshot));
    }

    private async flush(): Promise<void> {
        if (this.inFlight) return;
        const batch = [...this.entries]
            .filter(([, entry]) => this.isDue(entry))
            .slice(0, BATCH_SIZE);
        if (batch.length === 0) return;
        const captured = batch.map(([identifier, entry]) => ({
            identifier,
            entry,
            epoch: entry.epoch
        }));
        captured.forEach(({ entry }) => {
            entry.dirty = false;
        });
        this.inFlight = true;
        try {
            const snapshots = await this.load(captured.map(item => item.identifier));
            this.outageReported = false;
            const byIdentifier = new Map(snapshots.map(snapshot => [snapshot.asset_id, snapshot]));
            captured.forEach(({ identifier, entry, epoch }) =>
                this.publish(
                    identifier,
                    entry,
                    epoch,
                    byIdentifier.get(identifier) || {
                        asset_id: identifier,
                        state: 'unavailable',
                        generation: 0,
                        thumbnail_path: null
                    }
                )
            );
        } catch (error) {
            if (!this.outageReported)
                console.warn('Thumbnail processing status is unavailable:', error);
            this.outageReported = true;
            captured.forEach(({ identifier, entry, epoch }) =>
                this.publish(
                    identifier,
                    entry,
                    epoch,
                    entry.snapshot || {
                        asset_id: identifier,
                        state: 'unknown',
                        generation: 0,
                        thumbnail_path: null
                    },
                    true
                )
            );
        } finally {
            this.inFlight = false;
            this.scheduleNext();
        }
    }
}

const client = new ThumbnailProcessingClient(assetIds =>
    invoke<ThumbnailProcessingSnapshot[]>('get_thumbnail_processing_status', { assetIds })
);

/** Subscribes a mounted thumbnail to the shared batch client. */
export const subscribeThumbnailProcessing = (
    identifier: string,
    listener: Listener
): (() => void) => client.subscribe(identifier, listener);

/** Requests a fresh outcome after a generation or ready notification. */
export const invalidateThumbnailProcessing = (identifier: string): void =>
    client.invalidate(identifier);
