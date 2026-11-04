import { Index, Show, type Component } from 'solid-js';
import type {
    ProcessingClientState,
    WorkerKind,
    ProcessingRetryScope
} from '../../../core/store/processingTypes';
import type { ProgressPayload } from '../../../core/store/systemStore';
import { ProcessingWorkerRow } from './ProcessingWorkerRow';
import './processing-activity.css';
import { duplicateScanPhaseLabel } from '../../../utils/processing';

/** Snapshot presentation shared by the statusbar's activity popover. */
interface ProcessingActivityPanelProperties {
    /** Queues a user-selected terminal outcome group. */
    onRetry?: (worker: WorkerKind, scope: ProcessingRetryScope) => void;
    /** Shared transport state and errors. */
    state: ProcessingClientState;
    /** Existing indexer telemetry remains visible alongside controlled workers. */
    indexing: ProgressPayload | null;
    /** Dispatches a pause/resume action. */
    onPause: (worker: WorkerKind, paused: boolean) => void;
}

/** Provides explicit activity, drain state and control acknowledgement without notifications. */
export const ProcessingActivityPanel: Component<ProcessingActivityPanelProperties> = properties => (
    <section class="processing-activity" aria-label="System activity">
        <h3>System Activity</h3>
        <Show when={properties.state.error}>
            <p class="processing-error" role="status">
                {properties.state.error}
            </p>
        </Show>
        <Show when={properties.indexing}>
            <div class="processing-indexer">
                <strong>Indexing Library</strong>
                <span>
                    {properties.indexing?.processed} / {properties.indexing?.total}
                </span>
            </div>
        </Show>
        <Show
            when={properties.state.snapshot}
            fallback={
                <Show when={!properties.state.error}>
                    <p>Loading processing status…</p>
                </Show>
            }
        >
            <Index each={properties.state.snapshot?.workers}>
                {worker => (
                    <ProcessingWorkerRow
                        worker={worker()}
                        pending={properties.state.pending.has(worker().worker)}
                        onPause={properties.onPause}
                        onRetry={properties.onRetry}
                        comparisonFailures={
                            worker().worker === 'duplicates'
                                ? properties.state.snapshot?.matching?.failed
                                : 0
                        }
                        feedback={properties.state.feedback?.[worker().worker]}
                    />
                )}
            </Index>
            <Show when={properties.state.snapshot?.matching}>
                <dl class="processing-worker-counts" aria-label="Duplicate comparisons">
                    <div>
                        <dt>Comparisons pending</dt>
                        <dd>{properties.state.snapshot?.matching?.pending}</dd>
                    </div>
                    <div>
                        <dt>Comparisons retrying</dt>
                        <dd>{properties.state.snapshot?.matching?.retry_wait}</dd>
                    </div>
                    <div>
                        <dt>Comparisons failed</dt>
                        <dd>{properties.state.snapshot?.matching?.failed}</dd>
                    </div>
                </dl>
            </Show>
            <Show when={properties.state.snapshot?.manual_scan.is_running}>
                <div role="status">
                    <p>
                        Manual duplicate scan:{' '}
                        {duplicateScanPhaseLabel(properties.state.snapshot!.manual_scan.phase)}
                    </p>
                    <Show
                        when={
                            properties.state.snapshot?.manual_scan.phase === 'hashing' &&
                            properties.state.snapshot.manual_scan.total > 0
                        }
                    >
                        <p>
                            {properties.state.snapshot?.manual_scan.processed} /{' '}
                            {properties.state.snapshot?.manual_scan.total} files analyzed
                        </p>
                    </Show>
                </div>
            </Show>
        </Show>
        <p class="processing-help">
            Pause lets current operations finish. Thumbnail pause includes visible assets. Pauses
            reset when the app restarts.
        </p>
    </section>
);
