import { render } from '@solidjs/testing-library';
import type { JSX } from 'solid-js';
import type {
    ProcessingClientState,
    ProcessingRetryScope,
    WorkerKind,
    WorkerSnapshot
} from '../../../../core/store/processing/types';

export const createWorkerRow = (worker: WorkerSnapshot['worker']): WorkerSnapshot => ({
    worker,
    online: true,
    paused: false,
    active: 0,
    phase: null,
    scheduled: false,
    pending: 0,
    retry_wait: 0,
    failed: 0,
    skipped: 0,
    automatic_enabled: true
});

export const createInitialState = (): ProcessingClientState => ({
    snapshot: {
        instance_id: 'session',
        sequence: 1,
        workers: [
            createWorkerRow('thumbnails'),
            createWorkerRow('colors'),
            createWorkerRow('duplicates')
        ],
        manual_scan: { is_running: false, phase: 'idle', processed: 0, total: 0 }
    },
    error: null,
    pending: new Set()
});

export const createMatchingPayload = (failedCount: number = 1) => ({
    publication_sequence: 1,
    pending: 0,
    retry_wait: 0,
    failed: failedCount,
    completed: [],
    failed_phases: ['exact_matching', 'visual_matching'],
    manual_pending: false,
    manual_cancel_requested: false,
    manual_phase: null
});

export interface RenderActivityPanelHandlers {
    onCheckTools?: () => void;
    onRetry?: (worker: WorkerKind, scope: ProcessingRetryScope) => void;
    onPause?: (worker: WorkerKind, paused: boolean) => void;
    isDeveloperMode?: boolean;
}

export const renderWithComponent = (component: () => JSX.Element) => render(component);
