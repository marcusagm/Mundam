import { Show, type Component } from 'solid-js';
import { Copy, Image, Palette, Pause, Play } from 'lucide-solid';
import { Button } from '../../../ui/Button';
import { Badge } from '../../../ui/Badge';
import { ProgressBar } from '../../../ui/ProgressBar';
import type {
    WorkerKind,
    WorkerSnapshot,
    ProcessingRetryScope
} from '../../../../core/store/processing/types';
import {
    phaseLabel,
    workerNames,
    workerStatus,
    workerStatusVariant,
    humanWorkerPhaseLabel
} from './processingPresentation';
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
    /** Whether advanced developer counters and dl/dt/dd metrics are displayed. */
    isDeveloperMode?: boolean;
}

const renderWorkerTypeIcon = (workerKind: WorkerKind) => {
    if (workerKind === 'thumbnails') {
        return <Image size={15} class="processing-worker-type-icon" />;
    }
    if (workerKind === 'colors') {
        return <Palette size={15} class="processing-worker-type-icon" />;
    }
    if (workerKind === 'duplicates') {
        return <Copy size={15} class="processing-worker-type-icon" />;
    }
    return null;
};

const resolveWorkerBadgeLabel = (worker: WorkerSnapshot): string => {
    if (!worker.online) return 'Offline';
    if (worker.paused) return 'Paused';
    if (worker.active > 0) return 'Active';
    if (worker.failed > 0) return 'Errors';
    if ((worker.blocked ?? 0) > 0) return 'Waiting tools';
    if (worker.retry_wait > 0) return 'Retrying';
    if (worker.pending > 0 || worker.scheduled) return 'Queued';
    return 'Ready';
};

const hasActiveWorkerProgress = (worker: WorkerSnapshot, isDeveloper: boolean): boolean => {
    if (isDeveloper) return false;
    return worker.active > 0 || worker.pending > 0;
};

const shouldDisplayWorkerCountsGrid = (worker: WorkerSnapshot, isDeveloper: boolean): boolean => {
    return isDeveloper || worker.failed > 0 || (worker.blocked ?? 0) > 0;
};

const isDuplicateWorkerAutomaticOff = (worker: WorkerSnapshot): boolean => {
    return worker.worker === 'duplicates' && !worker.automatic_enabled;
};

/** Keeps operation progress separate from user pause intent and terminal failures. */
export const ProcessingWorkerRow: Component<ProcessingWorkerRowProperties> = properties => {
    const isDeveloper = () => properties.isDeveloperMode ?? false;

    return (
        <div class="processing-worker-card processing-worker-row">
            <div class="processing-worker-heading">
                <div class="processing-worker-title-group">
                    {renderWorkerTypeIcon(properties.worker.worker)}
                    <strong>{workerNames[properties.worker.worker]}</strong>
                </div>
                <div class="processing-worker-controls-group">
                    <Badge variant={workerStatusVariant(properties.worker)} size="sm">
                        {resolveWorkerBadgeLabel(properties.worker)}
                    </Badge>
                    <Button
                        variant="ghost"
                        size="icon-sm"
                        disabled={properties.pending || !properties.worker.online}
                        aria-busy={properties.pending}
                        aria-label={`${properties.worker.paused ? 'Resume' : 'Pause'} ${workerNames[properties.worker.worker]}`}
                        title={
                            properties.worker.paused ? 'Resume processing' : 'Pause new operations'
                        }
                        onClick={() =>
                            properties.onPause(properties.worker.worker, !properties.worker.paused)
                        }
                    >
                        <Show when={properties.worker.paused} fallback={<Pause size={14} />}>
                            <Play size={14} />
                        </Show>
                    </Button>
                </div>
            </div>

            <div class="processing-worker-status-line">
                <div class="processing-worker-status">{workerStatus(properties.worker)}</div>
                <Show when={properties.worker.scheduled && properties.worker.active === 0}>
                    <div class="processing-worker-scheduled-note">Comparison scheduled</div>
                </Show>
            </div>

            <Show when={properties.worker.active > 0}>
                <div class="processing-worker-phase">{phaseLabel(properties.worker.phase)}</div>
            </Show>

            <Show when={hasActiveWorkerProgress(properties.worker, isDeveloper())}>
                <div class="processing-worker-progress-block">
                    <div class="processing-worker-human-label">
                        {humanWorkerPhaseLabel(properties.worker.phase, properties.worker.worker)}
                    </div>
                    <ProgressBar
                        value={properties.worker.active > 0 ? 50 : 0}
                        isIndeterminate={properties.worker.active > 0}
                        size="sm"
                    />
                    <div class="processing-worker-summary-counts">
                        <span>{properties.worker.pending} remaining</span>
                        <Show when={properties.worker.active > 0}>
                            <span> • {properties.worker.active} active</span>
                        </Show>
                    </div>
                </div>
            </Show>

            <Show when={shouldDisplayWorkerCountsGrid(properties.worker, isDeveloper())}>
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
            </Show>

            <ProcessingRetryActions
                worker={properties.worker}
                comparisonFailures={properties.comparisonFailures}
                pending={properties.pending}
                onRetry={properties.onRetry}
            />

            <Show when={properties.feedback}>
                <p role="status" class="processing-help processing-feedback-note">
                    {properties.feedback}
                </p>
            </Show>

            <ProcessingDiagnostics diagnostics={properties.worker.diagnostics ?? []} />

            <Show when={(properties.comparisonFailures ?? 0) > 0}>
                <p class="processing-help">
                    Some duplicate comparisons failed. Retry queues comparisons without rebuilding
                    the index.
                </p>
            </Show>
            <Show when={isDuplicateWorkerAutomaticOff(properties.worker)}>
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
};
