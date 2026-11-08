import { Show, type Component } from 'solid-js';
import type { ProcessingSnapshot } from '../../../../core/store/processing/types';

interface ProcessingMatchingMatrixProperties {
    matching: ProcessingSnapshot['matching'];
}

/** Displays pairwise duplicate comparison metrics for developer diagnostics. */
export const ProcessingMatchingMatrix: Component<
    ProcessingMatchingMatrixProperties
> = properties => (
    <Show when={properties.matching}>
        <div class="processing-duplicates-matrix-card">
            <div class="processing-matrix-header">
                <strong>Duplicate comparisons</strong>
            </div>
            <dl class="processing-worker-counts" aria-label="Duplicate comparisons">
                <div>
                    <dt>Comparisons pending</dt>
                    <dd>{properties.matching?.pending}</dd>
                </div>
                <div>
                    <dt>Comparisons retrying</dt>
                    <dd>{properties.matching?.retry_wait}</dd>
                </div>
                <div>
                    <dt>Comparisons failed</dt>
                    <dd>{properties.matching?.failed}</dd>
                </div>
            </dl>
        </div>
    </Show>
);
