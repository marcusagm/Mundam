import { Component, createSignal, onMount } from 'solid-js';
import { Button, SectionGroup } from '../../ui';
import { Select } from '../../ui/Select';
import { useSystem, useSettings, useNotification } from '../../../core/hooks';
import { transcodeState, transcodeActions } from '../../../core/store/transcodeStore';
import { type TranscodeQuality } from '../../../lib/stream-utils';
import { formatFileSize } from '../../../utils/format';
import { TrashSettingsSection } from './TrashSettingsSection';
import './storage-panel.css';

/**
 * Settings panel dedicated to storage management, video transcoding cache,
 * library trash retention, folder recovery, and database optimization.
 *
 * @returns {JSX.Element} The rendered storage panel.
 */
export const StoragePanel: Component = () => {
    const system = useSystem();
    const settings = useSettings();
    const notification = useNotification();

    const [isOptimizingDatabase, setIsOptimizingDatabase] = createSignal(false);
    const [isCleaningOldCache, setIsCleaningOldCache] = createSignal(false);
    const [isClearingAllCache, setIsClearingAllCache] = createSignal(false);

    onMount(async () => {
        await settings.initialize();
        await settings.refreshCacheStats();
    });

    const handleOptimizeDatabase = async (): Promise<void> => {
        setIsOptimizingDatabase(true);
        notification.info('Starting database optimization...');
        try {
            await system.runDbMaintenance();
            notification.success('Database optimization complete.');
        } catch (error) {
            notification.error('Failed to optimize database.');
            console.error(error);
        } finally {
            setIsOptimizingDatabase(false);
        }
    };

    const handleRetentionChange = async (selectedDaysValue: string): Promise<void> => {
        const updateResult = await settings.updateSettings({
            cacheRetentionDays: Number(selectedDaysValue)
        });
        if (!updateResult.success) {
            notification.error('Failed to save settings.', updateResult.error.message);
        }
    };

    const handleCleanupOldCache = async (): Promise<void> => {
        setIsCleaningOldCache(true);
        try {
            const deletedCount = await system.cleanupCache();
            notification.success(`Cleaned up ${deletedCount} old cache files.`);
            await settings.refreshCacheStats();
        } catch (error) {
            notification.error('Failed to cleanup cache.');
            console.error(error);
        } finally {
            setIsCleaningOldCache(false);
        }
    };

    const handleClearAllCache = async (): Promise<void> => {
        setIsClearingAllCache(true);
        try {
            const deletedCount = await system.clearCache();
            notification.success(`Cleared ${deletedCount} cache files.`);
            await settings.refreshCacheStats();
        } catch (error) {
            notification.error('Failed to clear cache.');
            console.error(error);
        } finally {
            setIsClearingAllCache(false);
        }
    };

    const cacheRetentionOptions = [
        { value: '7', label: '7 days' },
        { value: '14', label: '14 days' },
        { value: '30', label: '30 days' },
        { value: '60', label: '60 days' },
        { value: '90', label: '90 days' }
    ];

    const playbackQualityOptions = [
        { value: 'preview', label: 'Preview (Faster, smaller files)' },
        { value: 'standard', label: 'Standard (Balanced)' },
        { value: 'high', label: 'High (Best quality, larger files)' }
    ];

    const handleQualityChange = (selectedQualityValue: string): void => {
        transcodeActions.setQuality(selectedQualityValue as TranscodeQuality);
        notification.success('Default quality updated.');
    };

    return (
        <div class="settings-panel-content storage-panel">
            <h2 class="settings-panel-title">Storage & Maintenance</h2>
            <p class="settings-panel-description">
                Manage cached media files, library trash retention, folder recovery, and database
                optimization to keep storage efficient.
            </p>

            <SectionGroup
                title="Transcoding Cache"
                description="Temporary audio and video files transcoded to ensure smooth playback in the library."
            >
                <div class="storage-stat-grid">
                    <div class="storage-stat-card">
                        <span class="storage-stat-label">Cached Files</span>
                        <span class="storage-stat-value">{settings.cacheStats().file_count}</span>
                    </div>
                    <div class="storage-stat-card">
                        <span class="storage-stat-label">Disk Footprint</span>
                        <span class="storage-stat-value">
                            {formatFileSize(settings.cacheStats().size_bytes)}
                        </span>
                    </div>
                </div>

                <div class="storage-setting-row">
                    <span class="setting-label">Auto-cleanup after:</span>
                    <div class="storage-control-container">
                        <Select
                            options={cacheRetentionOptions}
                            value={String(settings.cacheRetentionDays())}
                            onValueChange={handleRetentionChange}
                            placeholder="Select days"
                        />
                    </div>
                </div>

                <div class="storage-setting-row">
                    <span class="setting-label">Default Playback Quality:</span>
                    <div class="storage-control-container">
                        <Select
                            options={playbackQualityOptions}
                            value={transcodeState.quality()}
                            onValueChange={handleQualityChange}
                            placeholder="Select quality"
                        />
                    </div>
                </div>

                <div class="storage-actions-row">
                    <Button
                        onClick={handleCleanupOldCache}
                        loading={isCleaningOldCache()}
                        variant="outline"
                        size="sm"
                    >
                        Cleanup Old Files
                    </Button>
                    <Button
                        onClick={handleClearAllCache}
                        loading={isClearingAllCache()}
                        variant="destructive"
                        size="sm"
                    >
                        Clear All Cache
                    </Button>
                </div>
            </SectionGroup>

            <TrashSettingsSection />

            <SectionGroup
                title="Library Database Optimization"
                description="Reorganize database tables, analyze query indexes, and reclaim free disk space (VACUUM + ANALYZE)."
            >
                <div class="storage-actions-row">
                    <Button
                        onClick={handleOptimizeDatabase}
                        loading={isOptimizingDatabase()}
                        variant="outline"
                        size="sm"
                    >
                        Optimize Library Database
                    </Button>
                </div>
            </SectionGroup>
        </div>
    );
};
