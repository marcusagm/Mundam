import { Component, onMount } from 'solid-js';
import { SectionGroup, Switch } from '../../ui';
import { Input } from '../../ui/Input';
import { useSettings, useNotification } from '../../../core/hooks';
import { filterState, filterActions } from '../../../core/store/filter';
import './general-panel.css';

/**
 * Settings panel dedicated to general application preferences,
 * navigation history, and developer diagnostics.
 *
 * @returns {JSX.Element} The rendered general settings panel.
 */
export const GeneralPanel: Component = () => {
    const settings = useSettings();
    const notification = useNotification();

    onMount(async () => {
        await settings.initialize();
    });

    const handleDeveloperModeChange = async (checked: boolean): Promise<void> => {
        const updateResult = await settings.updateSettings({ developerMode: checked });
        if (updateResult.success) {
            notification.success('Settings Saved', 'Developer mode preference updated.');
        } else {
            notification.error('Error', updateResult.error.message);
        }
    };

    const handleHistoryLimitChange = (
        event: InputEvent & { currentTarget: HTMLInputElement }
    ): void => {
        const numericLimit = parseInt(event.currentTarget.value, 10);
        if (!isNaN(numericLimit) && numericLimit > 0) {
            filterActions.setHistoryLimit(numericLimit);
        }
    };

    return (
        <div class="settings-panel-content general-panel">
            <h2 class="settings-panel-title">General</h2>
            <p class="settings-panel-description">
                Configure browsing history, navigation preferences, and developer diagnostic tools.
            </p>

            <SectionGroup
                title="Browsing & Navigation"
                description="Control how navigation history and filter steps are retained."
            >
                <div class="general-setting-row">
                    <div class="general-setting-text-block">
                        <span class="setting-label">Filter History Limit</span>
                        <span class="general-setting-sublabel">
                            Number of recent search and filter states preserved for backwards and
                            forwards navigation.
                        </span>
                    </div>
                    <div style={{ width: '120px' }}>
                        <Input
                            type="number"
                            min="5"
                            max="100"
                            value={filterState.historyLimit}
                            onInput={handleHistoryLimitChange}
                        />
                    </div>
                </div>
            </SectionGroup>

            <SectionGroup
                title="Developer Diagnostics"
                description="Advanced telemetry and system observability tools for development and troubleshooting."
            >
                <div class="general-setting-row">
                    <div class="general-setting-text-block">
                        <span class="setting-label">Enable Developer Diagnostics</span>
                        <span class="general-setting-sublabel">
                            Display Tokio worker slots, raw processing queues, and media extraction
                            tools (FFmpeg, FFprobe, Assimp) status in the statusbar.
                        </span>
                    </div>
                    <Switch
                        aria-label="Enable Developer Diagnostics"
                        checked={settings.developerMode()}
                        onCheckedChange={handleDeveloperModeChange}
                    />
                </div>
            </SectionGroup>
        </div>
    );
};
