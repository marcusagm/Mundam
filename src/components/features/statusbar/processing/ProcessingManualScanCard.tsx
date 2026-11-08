import { Show, type Component } from 'solid-js';
import { ProgressBar } from '../../../ui/ProgressBar';
import { duplicateScanPhaseLabel } from '../../../../utils/processing';
import type { ProcessingSnapshot } from '../../../../core/store/processing/types';

interface ProcessingManualScanCardProperties {
    manualScan?: ProcessingSnapshot['manual_scan'];
}

/** Displays manual duplicate scanner progress and phase feedback. */
export const ProcessingManualScanCard: Component<
    ProcessingManualScanCardProperties
> = properties => {
    const isHashingWithTotal = () =>
        properties.manualScan?.phase === 'hashing' && (properties.manualScan?.total ?? 0) > 0;

    return (
        <Show when={properties.manualScan?.is_running}>
            <div class="processing-manual-scan-card" role="status">
                <p class="processing-manual-scan-title">
                    Manual duplicate scan: {duplicateScanPhaseLabel(properties.manualScan!.phase)}
                </p>
                <Show when={isHashingWithTotal()}>
                    <p class="processing-manual-scan-detail">
                        {properties.manualScan?.processed} / {properties.manualScan?.total} files
                        analyzed
                    </p>
                    <ProgressBar
                        value={properties.manualScan?.processed ?? 0}
                        maximumValue={properties.manualScan?.total ?? 100}
                        size="sm"
                    />
                </Show>
            </div>
        </Show>
    );
};
