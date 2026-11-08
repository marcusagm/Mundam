import { Show, type Component } from 'solid-js';
import { Button } from '../../../ui/Button';
import type {
    WorkerKind,
    WorkerSnapshot,
    ProcessingRetryScope
} from '../../../../core/store/processing/types';

/** Explicit terminal-outcome controls preserve worker policies and admission acknowledgement. */
interface ProcessingRetryActionsProperties {
    worker: WorkerSnapshot;
    comparisonFailures?: number;
    pending: boolean;
    onRetry?: (worker: WorkerKind, scope: ProcessingRetryScope) => void;
}

/** Keeps failed retries and unsupported reevaluation as separate user decisions. */
export const ProcessingRetryActions: Component<ProcessingRetryActionsProperties> = properties => {
    const isActionDisabled = () =>
        properties.pending || !properties.worker.online || !properties.onRetry;
    const totalFailedCount = () => properties.worker.failed + (properties.comparisonFailures ?? 0);

    return (
        <div class="processing-retry-actions">
            <Show when={totalFailedCount() > 0}>
                <Button
                    size="sm"
                    variant="outline"
                    disabled={isActionDisabled()}
                    onClick={() => properties.onRetry?.(properties.worker.worker, 'failed')}
                >
                    Retry failures
                </Button>
            </Show>
            <Show when={properties.worker.skipped > 0}>
                <Button
                    size="sm"
                    variant="outline"
                    disabled={isActionDisabled()}
                    onClick={() => properties.onRetry?.(properties.worker.worker, 'unsupported')}
                >
                    Recheck unsupported
                </Button>
            </Show>
        </div>
    );
};
