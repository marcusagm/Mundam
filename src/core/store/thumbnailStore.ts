import { createSignal } from "solid-js";
import { listen } from "@tauri-apps/api/event";

// Centralized store to track thumbnail regeneration state
// This persists across component mount/unmount cycles (virtualization)

interface ThumbnailReadyPayload {
  id: number;
  path: string;
}

interface RegenerationState {
  pending: Set<number>;           // IDs that are waiting for regeneration
  completed: Map<number, string>; // ID -> new thumbnail path
}

const [state, setState] = createSignal<RegenerationState>({
  pending: new Set(),
  completed: new Map(),
});

// Subscribers for thumbnail ready events
type ThumbnailCallback = (id: number, path: string) => void;
const subscribers = new Map<number, Set<ThumbnailCallback>>();

// Global listener - initialized once
let listenerInitialized = false;

async function initGlobalListener() {
  if (listenerInitialized) return;
  listenerInitialized = true;
  
  await listen<ThumbnailReadyPayload>("thumbnail:ready", (event) => {
    const { id, path } = event.payload;
    
    // Update store
    markRegenerationComplete(id, path);
    
    // Notify subscribers
    const callbacks = subscribers.get(id);
    if (callbacks) {
      callbacks.forEach(cb => cb(id, path));
    }
  });
}

// Initialize listener immediately
initGlobalListener();

export function subscribeThumbnailReady(imageId: number, callback: ThumbnailCallback): () => void {
  if (!subscribers.has(imageId)) {
    subscribers.set(imageId, new Set());
  }
  subscribers.get(imageId)!.add(callback);
  
  // Return unsubscribe function
  return () => {
    const callbacks = subscribers.get(imageId);
    if (callbacks) {
      callbacks.delete(callback);
      if (callbacks.size === 0) {
        subscribers.delete(imageId);
      }
    }
  };
}

export function markPendingRegeneration(imageId: number) {
  setState(s => {
    const newPending = new Set(s.pending);
    newPending.add(imageId);
    return { ...s, pending: newPending };
  });
}

export function markRegenerationComplete(imageId: number, thumbnailPath: string) {
  setState(s => {
    const newPending = new Set(s.pending);
    newPending.delete(imageId);
    const newCompleted = new Map(s.completed);
    newCompleted.set(imageId, thumbnailPath);
    return { pending: newPending, completed: newCompleted };
  });
}

export function isPendingRegeneration(imageId: number): boolean {
  return state().pending.has(imageId);
}

export function getCompletedThumbnail(imageId: number): string | undefined {
  return state().completed.get(imageId);
}

export function clearCompleted(imageId: number) {
  setState(s => {
    const newCompleted = new Map(s.completed);
    newCompleted.delete(imageId);
    return { ...s, completed: newCompleted };
  });
}
