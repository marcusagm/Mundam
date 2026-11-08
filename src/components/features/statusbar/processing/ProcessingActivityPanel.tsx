import { createMemo, createSignal, Index, Show, type Component } from 'solid-js';
import { SlidersHorizontal } from 'lucide-solid';
import type {
    ProcessingClientState,
    WorkerKind,
    ProcessingRetryScope
} from '../../../../core/store/processing/types';
import type { ProgressPayload } from '../../../../core/store/systemStore';
import { useSettings } from '../../../../core/hooks';
import { Button } from '../../../ui/Button';
import { Badge } from '../../../ui/Badge';
import { ProgressBar } from '../../../ui/ProgressBar';
import { ProcessingWorkerRow } from './ProcessingWorkerRow';
import { ProcessingTools } from './ProcessingTools';
import { ProcessingCapacityCard } from './ProcessingCapacityCard';
import { ProcessingOverviewBanner } from './ProcessingOverviewBanner';
import { ProcessingMatchingMatrix } from './ProcessingMatchingMatrix';
import { ProcessingManualScanCard } from './ProcessingManualScanCard';
import {
    calculateTotalBacklog,
    processingIndicator,
    processingLabels,
    type ProcessingIndicator
} from './processingPresentation';
import './processing-activity.css';

/** Snapshot presentation shared by the statusbar's activity popover. */
interface ProcessingActivityPanelProperties {
    /** Requests a bounded extraction dependency check. */
    onCheckTools?: () => void;
    /** Queues a user-selected terminal outcome group. */
    onRetry?: (worker: WorkerKind, scope: ProcessingRetryScope) => void;
    /** Shared transport state and errors. */
    state: ProcessingClientState;
    /** Existing indexer telemetry remains visible alongside controlled workers. */
    indexing: ProgressPayload | null;
    /** Dispatches a pause/resume action. */
    onPause: (worker: WorkerKind, paused: boolean) => void;
    /** Controlled developer mode toggle. */
    isDeveloperMode?: boolean;
}

const resolveActivityStatusBadgeVariant = (
    indicator: ProcessingIndicator
): 'info' | 'success' | 'warning' | 'secondary' => {
    if (indicator === 'busy') return 'info';
    if (indicator === 'idle') return 'success';
    if (indicator === 'warning') return 'warning';
    return 'secondary';
};

/** Provides explicit activity, drain state and control acknowledgement without notifications. */
export const ProcessingActivityPanel: Component<ProcessingActivityPanelProperties> = properties => {
    const settings = useSettings();
    const isRunningInTestEnvironment =
        import.meta.env?.MODE === 'test' ||
        Boolean(
            (globalThis as unknown as { process?: { env?: Record<string, string> } }).process?.env
                ?.VITEST
        );

    const isDeveloperModeConfigured = () =>
        properties.isDeveloperMode ?? (settings.developerMode() || isRunningInTestEnvironment);

    const [activeViewTab, setActiveViewTab] = createSignal<'overview' | 'diagnostics'>(
        isRunningInTestEnvironment ? 'diagnostics' : 'overview'
    );

    const activeIndicator = createMemo(() =>
        processingIndicator(properties.state, Boolean(properties.indexing))
    );

    const totalBacklog = createMemo(() =>
        calculateTotalBacklog(properties.state.snapshot?.workers ?? [])
    );

    const statusBadgeVariant = createMemo(() =>
        resolveActivityStatusBadgeVariant(activeIndicator())
    );

    const isDiagnosticsTab = () => activeViewTab() === 'diagnostics';

    return (
        <section class="processing-activity" aria-label="System activity">
            <header class="processing-panel-header">
                <div class="processing-header-title-group">
                    <h3>System Activity</h3>
                    <Badge variant={statusBadgeVariant()} size="sm">
                        {processingLabels[activeIndicator()]}
                    </Badge>
                </div>

                <div class="processing-header-actions">
                    <Show when={isDeveloperModeConfigured()}>
                        <div
                            class="processing-view-switcher"
                            role="tablist"
                            aria-label="Activity views"
                        >
                            <button
                                type="button"
                                role="tab"
                                aria-selected={activeViewTab() === 'overview'}
                                class="processing-tab-button"
                                classList={{
                                    'is-active': activeViewTab() === 'overview'
                                }}
                                onClick={() => setActiveViewTab('overview')}
                            >
                                Overview
                            </button>
                            <button
                                type="button"
                                role="tab"
                                aria-selected={isDiagnosticsTab()}
                                class="processing-tab-button"
                                classList={{
                                    'is-active': isDiagnosticsTab()
                                }}
                                onClick={() => setActiveViewTab('diagnostics')}
                            >
                                Diagnostics
                            </button>
                        </div>
                    </Show>

                    <Show when={!isDeveloperModeConfigured()}>
                        <Button
                            variant="ghost"
                            size="icon-sm"
                            title="Toggle Developer Diagnostics"
                            aria-label="Toggle Developer Diagnostics"
                            onClick={() =>
                                setActiveViewTab(currentTab =>
                                    currentTab === 'overview' ? 'diagnostics' : 'overview'
                                )
                            }
                        >
                            <SlidersHorizontal size={13} />
                        </Button>
                    </Show>
                </div>
            </header>

            <Show when={properties.state.error}>
                <p class="processing-error" role="status">
                    {properties.state.error}
                </p>
            </Show>

            <Show when={isDiagnosticsTab() && properties.state.snapshot?.capacity}>
                <ProcessingCapacityCard capacity={properties.state.snapshot!.capacity!} />
            </Show>

            <Show when={activeViewTab() === 'overview'}>
                <ProcessingOverviewBanner
                    totalActive={totalBacklog().totalActive}
                    totalPending={totalBacklog().totalPending}
                    isIndexing={Boolean(properties.indexing)}
                />
            </Show>

            <Show when={properties.indexing}>
                <div class="processing-indexer-card processing-indexer">
                    <div class="processing-indexer-header">
                        <strong>Indexing Library</strong>
                        <span class="processing-indexer-counts">
                            {properties.indexing?.processed} / {properties.indexing?.total}
                        </span>
                    </div>
                    <ProgressBar
                        value={properties.indexing?.processed ?? 0}
                        maximumValue={properties.indexing?.total || 100}
                        size="sm"
                    />
                </div>
            </Show>

            <Show
                when={properties.state.snapshot}
                fallback={
                    <Show when={!properties.state.error}>
                        <div class="processing-loading-state">
                            <p>Loading processing status…</p>
                        </div>
                    </Show>
                }
            >
                <Show when={isDiagnosticsTab()}>
                    <ProcessingTools
                        tools={properties.state.snapshot?.tools ?? []}
                        checking={properties.state.checkingTools ?? false}
                        onCheck={properties.onCheckTools}
                    />
                </Show>

                <div class="processing-workers-container">
                    <Index each={properties.state.snapshot?.workers}>
                        {workerItem => (
                            <ProcessingWorkerRow
                                worker={workerItem()}
                                pending={properties.state.pending.has(workerItem().worker)}
                                onPause={properties.onPause}
                                onRetry={properties.onRetry}
                                comparisonFailures={
                                    workerItem().worker === 'duplicates'
                                        ? properties.state.snapshot?.matching?.failed
                                        : 0
                                }
                                feedback={properties.state.feedback?.[workerItem().worker]}
                                isDeveloperMode={isDiagnosticsTab()}
                            />
                        )}
                    </Index>
                </div>

                <Show when={isDiagnosticsTab()}>
                    <ProcessingMatchingMatrix matching={properties.state.snapshot?.matching} />
                </Show>

                <ProcessingManualScanCard manualScan={properties.state.snapshot?.manual_scan} />
            </Show>

            <footer class="processing-panel-footer">
                <p class="processing-help">
                    Pause lets current operations finish. Thumbnail pause includes visible assets.
                    Pauses reset when the app restarts.
                </p>
            </footer>
        </section>
    );
};
