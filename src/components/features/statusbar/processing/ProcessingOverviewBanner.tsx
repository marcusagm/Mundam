import { Show, type Component } from 'solid-js';
import { Activity, CheckCircle2 } from 'lucide-solid';
import { ProgressBar } from '../../../ui/ProgressBar';

interface ProcessingOverviewBannerProperties {
    totalActive: number;
    totalPending: number;
    isIndexing: boolean;
}

/** Shows consolidated user-facing media operations progress or complete state. */
export const ProcessingOverviewBanner: Component<
    ProcessingOverviewBannerProperties
> = properties => {
    const isWorking = () =>
        properties.totalActive > 0 || properties.totalPending > 0 || properties.isIndexing;

    return (
        <div class="processing-overview-banner">
            <Show
                when={isWorking()}
                fallback={
                    <div class="processing-overview-idle">
                        <CheckCircle2 size={18} class="processing-idle-icon" />
                        <div>
                            <div class="processing-overview-idle-title">Library is up to date</div>
                            <div class="processing-overview-idle-subtitle">
                                All background media operations are completed.
                            </div>
                        </div>
                    </div>
                }
            >
                <div class="processing-overview-active">
                    <div class="processing-overview-active-header">
                        <div class="processing-overview-active-title">
                            <Activity size={14} class="processing-pulse-icon" />
                            <span>Processing media library</span>
                        </div>
                        <span class="processing-overview-count">
                            {properties.totalPending} remaining
                        </span>
                    </div>
                    <ProgressBar
                        value={properties.totalActive > 0 ? 50 : 0}
                        isIndeterminate={properties.totalActive > 0}
                        size="sm"
                    />
                </div>
            </Show>
        </div>
    );
};
