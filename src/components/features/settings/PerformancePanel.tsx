import { Component, Show } from 'solid-js';
import { PerformanceSettings } from './PerformanceSettings';
import { LinuxGraphicsSettings } from './LinuxGraphicsSettings';
import { LinuxVideoSettings } from './LinuxVideoSettings';
import { detectPlatform } from '../../../core/input/utils/platform';

/**
 * Settings panel dedicated to hardware allocation, background processing concurrency,
 * and library indexing throughput.
 *
 * @returns {JSX.Element} The rendered performance settings panel.
 */
export const PerformancePanel: Component = () => {
    return (
        <div class="settings-panel-content performance-panel">
            <h2 class="settings-panel-title">Performance</h2>
            <p class="settings-panel-description">
                Configure background worker capacity, thumbnail generation threads, and disk
                indexing throughput to balance system responsiveness with library operations.
            </p>

            <PerformanceSettings />
            <Show when={detectPlatform() === 'linux'}>
                <LinuxGraphicsSettings />
                <LinuxVideoSettings />
            </Show>
        </div>
    );
};
