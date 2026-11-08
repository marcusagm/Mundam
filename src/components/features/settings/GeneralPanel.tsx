import { Button, SectionGroup, Switch } from '../../ui';
import { Component, createSignal, onMount } from 'solid-js';
import { Select } from '../../ui/Select';
import { Input } from '../../ui/Input';
import { useSystem, useSettings, useNotification } from '../../../core/hooks';
import { filterState, filterActions } from '../../../core/store/filter';
import { transcodeState, transcodeActions } from '../../../core/store/transcodeStore';
import { type TranscodeQuality } from '../../../lib/stream-utils';
import { TrashSettingsSection } from './TrashSettingsSection';
import { PerformanceSettings } from './PerformanceSettings';
import './general-panel.css';

import { formatFileSize } from '../../../utils/format';

export const GeneralPanel: Component = () => {
    const system = useSystem();
    const settings = useSettings();
    const notification = useNotification();

    const [optimizing, setOptimizing] = createSignal(false);
    const [cleaningCache, setCleaningCache] = createSignal(false);
    const [clearingCache, setClearingCache] = createSignal(false);

    onMount(async () => {
        await settings.initialize();
    });

    const handleOptimize = async () => {
        setOptimizing(true);
        notification.info('Starting database optimization...');
        try {
            await system.runDbMaintenance();
            notification.success('Database optimization complete.');
        } catch (error) {
            notification.error('Failed to optimize database.');
            console.error(error);
        } finally {
            setOptimizing(false);
        }
    };

    const handleRetentionChange = async (value: string) => {
        const result = await settings.updateSettings({ cacheRetentionDays: Number(value) });
        if (!result.success) {
            notification.error('Failed to save settings.', result.error.message);
        }
    };

    const handleCleanupCache = async () => {
        setCleaningCache(true);
        try {
            const deleted = await system.cleanupCache();
            notification.success(`Cleaned up ${deleted} old cache files.`);
            await settings.refreshCacheStats();
        } catch (error) {
            notification.error('Failed to cleanup cache.');
            console.error(error);
        } finally {
            setCleaningCache(false);
        }
    };

    const handleClearCache = async () => {
        setClearingCache(true);
        try {
            const deleted = await system.clearCache();
            notification.success(`Cleared ${deleted} cache files.`);
            await settings.refreshCacheStats();
        } catch (error) {
            notification.error('Failed to clear cache.');
            console.error(error);
        } finally {
            setClearingCache(false);
        }
    };

    const retentionOptions = [
        { value: '7', label: '7 days' },
        { value: '14', label: '14 days' },
        { value: '30', label: '30 days' },
        { value: '60', label: '60 days' },
        { value: '90', label: '90 days' }
    ];

    const qualityOptions = [
        { value: 'preview', label: 'Preview (Faster, smaller files)' },
        { value: 'standard', label: 'Standard (Balanced)' },
        { value: 'high', label: 'High (Best quality, larger files)' }
    ];

    const handleQualityChange = (value: string) => {
        transcodeActions.setQuality(value as TranscodeQuality);
        notification.success('Default quality updated.');
    };

    const handleDeveloperModeChange = async (checked: boolean): Promise<void> => {
        const updateResult = await settings.updateSettings({ developerMode: checked });
        if (updateResult.success) {
            notification.success('Settings Saved', 'Developer mode preference updated.');
        } else {
            notification.error('Error', updateResult.error.message);
        }
    };

    return (
        <div class="settings-panel-content general-panel">
            <h2 class="settings-panel-title">General</h2>

            <PerformanceSettings />

            <SectionGroup
                title="Transcoding Cache"
                description="Manage cached video/audio files that were transcoded for playback."
            >
                <div class="cache-stats">
                    <div class="cache-stat-item">
                        <span class="cache-stat-label">Files:</span>
                        <span class="cache-stat-value">{settings.cacheStats().file_count}</span>
                    </div>
                    <div class="cache-stat-item">
                        <span class="cache-stat-label">Size:</span>
                        <span class="cache-stat-value">
                            {formatFileSize(settings.cacheStats().size_bytes)}
                        </span>
                    </div>
                </div>
                <div class="general-setting-row">
                    <span class="setting-label">Auto-cleanup after:</span>
                    <div style={{ width: '140px' }}>
                        <Select
                            options={retentionOptions}
                            value={String(settings.cacheRetentionDays())}
                            onValueChange={handleRetentionChange}
                            placeholder="Select days"
                        />
                    </div>
                </div>
                <div class="general-setting-row">
                    <span class="setting-label">Default Quality:</span>
                    <div style={{ width: '240px' }}>
                        <Select
                            options={qualityOptions}
                            value={transcodeState.quality()}
                            onValueChange={handleQualityChange}
                            placeholder="Select quality"
                        />
                    </div>
                </div>
                <div class="setting-action-row cache-actions">
                    <Button
                        onClick={handleCleanupCache}
                        loading={cleaningCache()}
                        variant="outline"
                    >
                        Cleanup Old Files
                    </Button>
                    <Button
                        onClick={handleClearCache}
                        loading={clearingCache()}
                        variant="destructive"
                    >
                        Clear All Cache
                    </Button>
                </div>
            </SectionGroup>

            <SectionGroup title="Browsing" description="Configure your navigation experience.">
                <div class="general-setting-row">
                    <span class="setting-label">History Limit:</span>
                    <div style={{ width: '200px' }}>
                        <Input
                            type="number"
                            value={filterState.historyLimit}
                            onInput={e => {
                                const value = parseInt(e.currentTarget.value);
                                if (!isNaN(value) && value > 0) {
                                    filterActions.setHistoryLimit(value);
                                }
                            }}
                        />
                    </div>
                </div>
            </SectionGroup>

            <TrashSettingsSection />

            <SectionGroup
                title="Developer Mode"
                description="Configure diagnostic tools and advanced system telemetry."
            >
                <div class="general-setting-row">
                    <div
                        style={{
                            display: 'flex',
                            'flex-direction': 'column',
                            gap: 'var(--p-space-xxs)'
                        }}
                    >
                        <span class="setting-label">Enable Developer Diagnostics</span>
                        <span
                            style={{
                                'font-size': 'var(--p-font-size-xxs)',
                                color: 'var(--text-tertiary)'
                            }}
                        >
                            Show Tokio concurrency slots, extraction tool status, and raw worker
                            queues in the statusbar.
                        </span>
                    </div>
                    <Switch
                        aria-label="Enable Developer Diagnostics"
                        checked={settings.developerMode()}
                        onCheckedChange={handleDeveloperModeChange}
                    />
                </div>
            </SectionGroup>

            <SectionGroup
                title="Library Maintenance"
                description="Optimize the database to improve performance and reduce file size (VACUUM + ANALYZE)."
            >
                <div class="setting-action-row">
                    <Button onClick={handleOptimize} loading={optimizing()}>
                        Optimize Library
                    </Button>
                </div>
            </SectionGroup>
        </div>
    );
};
