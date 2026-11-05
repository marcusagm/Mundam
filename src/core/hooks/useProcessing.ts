import { createSignal, onCleanup, onMount } from 'solid-js';
import { invoke } from '@tauri-apps/api/core';
import { ProcessingClient } from '../store/processing/client';
import type {
    ProcessingClientState,
    ProcessingSnapshot,
    ProcessingRetryAcknowledgement
} from '../store/processing/types';

const client = new ProcessingClient({
    checkTools: () => invoke<ProcessingSnapshot>('check_processing_tools'),
    retry: (worker, scope) =>
        invoke<ProcessingRetryAcknowledgement>('retry_processing_worker', { worker, scope }),
    snapshot: () => invoke<ProcessingSnapshot>('get_processing_snapshot'),
    setPaused: (worker, paused) =>
        invoke<ProcessingSnapshot>('set_processing_worker_paused', { worker, paused })
});

/** Shares status transport while binding consumer cleanup to Solid's lifecycle. */
export function useProcessing() {
    const [state, setState] = createSignal<ProcessingClientState>({
        snapshot: null,
        error: null,
        pending: new Set()
    });
    let release: (() => void) | undefined;
    onMount(() => {
        release = client.subscribe(setState);
    });
    onCleanup(() => release?.());
    return {
        /** Last snapshot, query errors and pending controls. */
        state,
        /** Rechecks extraction dependencies without changing pause intent. */
        checkTools: () => client.checkTools(),
        /** Queues terminal outcomes after an explicit user action. */
        retry: client.retry.bind(client),
        /** Requests reconciliation after an explicit user action. */
        refresh: () => client.refresh(),
        /** Sends pause/resume intent without optimistic execution state. */
        setPaused: client.setPaused.bind(client)
    };
}
