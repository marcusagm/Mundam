import { createSignal } from 'solid-js';
import { listen } from '@tauri-apps/api/event';
import { invalidateThumbnailProcessing } from './thumbnailProcessing';

// Centralized store to track thumbnail regeneration state
// This persists across component mount/unmount cycles (virtualization)

interface ThumbnailReadyPayload {
    id: string;
    path: string;
}

interface RegenerationState {
    pending: Set<string>; // IDs that are waiting for regeneration
    completed: Map<string, string>; // ID -> new thumbnail path
}

const [state, setState] = createSignal<RegenerationState>({
    pending: new Set(),
    completed: new Map()
});

// Subscribers for thumbnail ready events
type ThumbnailCallback = (id: string, path: string) => void;
const subscribers = new Map<string, Set<ThumbnailCallback>>();

// Global listener - initialized once
let listenerInitialized = false;

async function initGlobalListener() {
    if (listenerInitialized) return;
    listenerInitialized = true;

    try {
        await listen<ThumbnailReadyPayload>('thumbnail:ready', event => {
            const { id, path } = event.payload;

            // Update store
            markRegenerationComplete(id, path);

            // Notify subscribers
            const callbacks = subscribers.get(id);
            if (callbacks) {
                callbacks.forEach(cb => cb(id, path));
            }
            invalidateThumbnailProcessing(id);
        });
    } catch (error) {
        listenerInitialized = false;
        console.warn(
            'Thumbnail ready listener is unavailable; status snapshots remain recoverable:',
            error
        );
    }
}

// Initialize listener immediately
initGlobalListener();

export function subscribeThumbnailReady(assetId: string, callback: ThumbnailCallback): () => void {
    void initGlobalListener();
    if (!subscribers.has(assetId)) {
        subscribers.set(assetId, new Set());
    }
    subscribers.get(assetId)!.add(callback);

    // Return unsubscribe function
    return () => {
        const callbacks = subscribers.get(assetId);
        if (callbacks) {
            callbacks.delete(callback);
            if (callbacks.size === 0) {
                subscribers.delete(assetId);
            }
        }
    };
}

export function markPendingRegeneration(assetId: string) {
    setState(s => {
        const newPending = new Set(s.pending);
        newPending.add(assetId);
        const completed = new Map(s.completed);
        completed.delete(assetId);
        return { completed, pending: newPending };
    });
}

export function markRegenerationComplete(assetId: string, thumbnailPath: string) {
    setState(s => {
        const newPending = new Set(s.pending);
        newPending.delete(assetId);
        const newCompleted = new Map(s.completed);
        newCompleted.set(assetId, thumbnailPath);
        while (newCompleted.size > 1000) {
            const oldest = newCompleted.keys().next().value;
            if (oldest !== undefined) newCompleted.delete(oldest);
        }
        return { pending: newPending, completed: newCompleted };
    });
}

/** Clears waiting state after a terminal extraction outcome or rejected regeneration request. */
export function clearPendingRegeneration(assetId: string): void {
    if (!state().pending.has(assetId)) return;
    setState(previous => {
        const pending = new Set(previous.pending);
        pending.delete(assetId);
        return { ...previous, pending };
    });
}

export function isPendingRegeneration(assetId: string): boolean {
    return state().pending.has(assetId);
}

export function getCompletedThumbnail(assetId: string): string | undefined {
    return state().completed.get(assetId);
}

export function clearCompleted(assetId: string) {
    setState(s => {
        const newCompleted = new Map(s.completed);
        newCompleted.delete(assetId);
        return { ...s, completed: newCompleted };
    });
}
