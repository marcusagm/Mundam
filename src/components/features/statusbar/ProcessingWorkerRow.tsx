import { Show, type Component } from 'solid-js';
import { Pause, Play } from 'lucide-solid';
import { Button } from '../../ui/Button';
import type { WorkerKind, WorkerSnapshot } from '../../../core/store/processingTypes';
import { phaseLabel, workerNames, workerStatus } from './processingPresentation';

/** One controlled executor and its distinct queue outcomes. */
interface ProcessingWorkerRowProperties {
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
                disabled={properties.pending}
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
            <div>
                <dt>Failed</dt>
                <dd>{properties.worker.failed}</dd>
            </div>
            <div>
                <dt>Unsupported</dt>
                <dd>{properties.worker.skipped}</dd>
            </div>
        </dl>
    </div>
);
