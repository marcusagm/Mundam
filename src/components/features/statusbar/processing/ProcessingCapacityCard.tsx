import { type Component } from 'solid-js';
import type { ProcessingCapacity } from '../../../../core/store/processing/types';

interface ProcessingCapacityCardProperties {
    capacity: ProcessingCapacity;
}

/** Displays Tokio scheduler capacity and thumbnail limits for developer diagnostics. */
export const ProcessingCapacityCard: Component<ProcessingCapacityCardProperties> = properties => (
    <div class="processing-capacity-card" aria-label="Processing capacity">
        <div class="processing-capacity-header">
            <span class="processing-capacity-title">Concurrency & Slot Allocation</span>
            <span class="processing-capacity-badge">
                {properties.capacity.active} / {properties.capacity.limit} active
            </span>
        </div>
        <div class="processing-capacity-stats-grid">
            <div class="processing-capacity-stat">
                <span class="processing-stat-label">Active operations</span>
                <span class="processing-stat-value">{properties.capacity.active}</span>
            </div>
            <div class="processing-capacity-stat">
                <span class="processing-stat-label">Waiting for capacity</span>
                <span class="processing-stat-value">{properties.capacity.waiting}</span>
            </div>
            <div class="processing-capacity-stat">
                <span class="processing-stat-label">Thumbnail limit</span>
                <span class="processing-stat-value">{properties.capacity.thumbnail_limit}</span>
            </div>
        </div>
        <p class="processing-help processing-capacity-help">
            {properties.capacity.active} / {properties.capacity.limit} media operations active;{' '}
            {properties.capacity.waiting} waiting for capacity. Thumbnail limit:{' '}
            {properties.capacity.thumbnail_limit}. Lower limits apply as current operations finish.
        </p>
    </div>
);
