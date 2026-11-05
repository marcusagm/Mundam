import { Show, type Component } from 'solid-js';
import { Pause, Play } from 'lucide-solid';
import { Button } from '../../../ui/Button';
import type {
    WorkerKind,
    WorkerSnapshot,
    ProcessingRetryScope
} from '../../../../core/store/processing/types';
import { phaseLabel, workerNames, workerStatus } from './processingPresentation';
import { ProcessingRetryActions } from './ProcessingRetryActions';
import { ProcessingDiagnostics } from './ProcessingDiagnostics';

/** One controlled executor and its distinct queue outcomes. */
interface ProcessingWorkerRowProperties {
    /** Failed comparisons share the duplicates worker's retry control. */
    comparisonFailures?: number;
    /** Acknowledgement of an explicit queue request. */
    feedback?: string;
    /** Optional for read-only consumers. */
    onRetry?: (worker: WorkerKind, scope: ProcessingRetryScope) => void;
    /** Authoritative worker snapshot. */
    worker: WorkerSnapshot;
    /** Whether its action is awaiting acknowledgement. */
    pending: boolean;
    /** Sends the requested pause state. */
    onPause: (worker: WorkerKind, paused: boolean) => void;
}

/** Keeps operation progress separate from user pause intent and terminal failures. */
export const ProcessingWorkerRow: Component<ProcessingWorkerRowProperties> = properties => (
    <div class="processing-worker-row">
        <div class="processing-worker-heading">
            <strong>{workerNames[properties.worker.worker]}</strong>
            <Button
                variant="ghost"
                size="icon-sm"
                disabled={properties.pending || !properties.worker.online}
                aria-busy={properties.pending}
                aria-label={`${properties.worker.paused ? 'Resume' : 'Pause'} ${workerNames[properties.worker.worker]}`}
                title={properties.worker.paused ? 'Resume processing' : 'Pause new operations'}
                onClick={() =>
                    properties.onPause(properties.worker.worker, !properties.worker.paused)
                }
            >
                <Show when={properties.worker.paused} fallback={<Pause size={14} />}>
                    <Play size={14} />
                </Show>
            </Button>
        </div>
        <div class="processing-worker-status">{workerStatus(properties.worker)}</div>
        <Show when={properties.worker.scheduled && properties.worker.active === 0}>
            <div>Comparison scheduled</div>
        </Show>
        <Show when={properties.worker.active > 0}>
            <div>{phaseLabel(properties.worker.phase)}</div>
        </Show>
        <dl class="processing-worker-counts">
            <div>
                <dt>Active</dt>
                <dd>{properties.worker.active}</dd>
            </div>
            <div>
                <dt>Pending</dt>
                <dd>{properties.worker.pending}</dd>
            </div>
            <div>
                <dt>Retries</dt>
                <dd>{properties.worker.retry_wait}</dd>
            </div>
            <Show when={(properties.worker.blocked ?? 0) > 0}>
                <div>
                    <dt>Waiting for tools</dt>
                    <dd>{properties.worker.blocked}</dd>
                </div>
            </Show>
            <div>
                <dt>Failed</dt>
                <dd>{properties.worker.failed}</dd>
            </div>
            <div>
                <dt>Unsupported</dt>
                <dd>{properties.worker.skipped}</dd>
            </div>
        </dl>
        <ProcessingRetryActions
            worker={properties.worker}
            comparisonFailures={properties.comparisonFailures}
            pending={properties.pending}
            onRetry={properties.onRetry}
        />
        <Show when={properties.feedback}>
            <p role="status" class="processing-help">
                {properties.feedback}
            </p>
        </Show>
        <ProcessingDiagnostics diagnostics={properties.worker.diagnostics ?? []} />
        <Show when={(properties.comparisonFailures ?? 0) > 0}>
            <p class="processing-help">
                Some duplicate comparisons failed. Retry queues comparisons without rebuilding the
                index.
            </p>
        </Show>
        <Show
            when={properties.worker.worker === 'duplicates' && !properties.worker.automatic_enabled}
        >
            <p class="processing-help">
                Automatic detection is off. Enable it in settings or use Scan Now in Duplicate
                Finder to process queued work.
            </p>
        </Show>
        <Show when={properties.worker.failed > 0}>
            <p class="processing-help">
                Some files reached the retry limit. A new scan does not reset these failures.
            </p>
        </Show>
        <Show when={properties.worker.skipped > 0}>
            <p class="processing-help">
                Some files cannot be processed with the available extractors.
            </p>
        </Show>
    </div>
);
