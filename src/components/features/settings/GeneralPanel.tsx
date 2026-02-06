import { Component, createSignal, onMount } from 'solid-js';
import { Button } from '../../ui/Button';
import { toast } from '../../ui/Sonner';
import { SectionGroup } from '../../ui/SectionGroup';
import { Select } from '../../ui/Select';
import { Input } from '../../ui/Input';
import { tauriService } from '../../../core/tauri/services';
import { filterState, filterActions } from '../../../core/store/filterStore';
import { transcodeState, transcodeActions } from '../../../core/store/transcodeStore';
import { type TranscodeQuality } from '../../../lib/stream-utils';
import './general-panel.css';

// Helper to format bytes
function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

export const GeneralPanel: Component = () => {
    const [optimizing, setOptimizing] = createSignal(false);
    const [threads, setThreads] = createSignal<string>('2');
    const [cacheRetentionDays, setCacheRetentionDays] = createSignal<string>('30');
    const [cleaningCache, setCleaningCache] = createSignal(false);
    const [clearingCache, setClearingCache] = createSignal(false);
    const [cacheStats, setCacheStats] = createSignal<{ size_bytes: number; file_count: number }>({
        size_bytes: 0,
        file_count: 0
    });

    onMount(async () => {
        // Load settings
        const threadVal = await tauriService.getSetting('thumbnail_threads');
        if (threadVal !== null && threadVal !== undefined) setThreads(String(threadVal));

        const retentionVal = await tauriService.getSetting('cache_retention_days');
        if (retentionVal !== null && retentionVal !== undefined)
            setCacheRetentionDays(String(retentionVal));

        // Load cache stats
        const stats = await tauriService.getCacheStats();
        setCacheStats({ size_bytes: stats.size_bytes, file_count: stats.file_count });
    });

    const handleOptimize = async () => {
        setOptimizing(true);
        toast.info('Starting database optimization...');
        try {
            await tauriService.runDbMaintenance();
            toast.success('Database optimization complete.');
        } catch (e) {
            toast.error('Failed to optimize database.');
            console.error(e);
        } finally {
            setOptimizing(false);
        }
    };

    const handleThreadChange = async (val: string) => {
        setThreads(val);
        try {
            await tauriService.setSetting('thumbnail_threads', parseInt(val));
            toast.success('Settings saved. Please restart the app for changes to take effect.');
        } catch (e) {
            toast.error('Failed to save settings.');
        }
    };

    const handleRetentionChange = async (val: string) => {
        setCacheRetentionDays(val);
        const days = parseInt(val);
        if (!isNaN(days) && days > 0) {
            try {
                await tauriService.setSetting('cache_retention_days', days);
            } catch (e) {
                toast.error('Failed to save settings.');
            }
        }
    };

    const handleCleanupCache = async () => {
        setCleaningCache(true);
        try {
            const days = parseInt(cacheRetentionDays()) || 30;
            const deleted = await tauriService.cleanupCache(days);
            toast.success(`Cleaned up ${deleted} old cache files.`);
            // Refresh stats
            const stats = await tauriService.getCacheStats();
            setCacheStats({ size_bytes: stats.size_bytes, file_count: stats.file_count });
        } catch (e) {
            toast.error('Failed to cleanup cache.');
            console.error(e);
        } finally {
            setCleaningCache(false);
        }
    };

    const handleClearCache = async () => {
        setClearingCache(true);
        try {
            const deleted = await tauriService.clearCache();
            toast.success(`Cleared ${deleted} cache files.`);
            setCacheStats({ size_bytes: 0, file_count: 0 });
        } catch (e) {
            toast.error('Failed to clear cache.');
            console.error(e);
        } finally {
            setClearingCache(false);
        }
    };

    const threadOptions = [
        { value: '0', label: 'Auto-Detect (Recommended)' },
        { value: '1', label: '1 (Low CPU)' },
        { value: '2', label: '2 (Balanced)' },
        { value: '4', label: '4 (High Performance)' },
        { value: '8', label: '8 (Extreme)' }
    ];

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

    const handleQualityChange = (val: string) => {
        transcodeActions.setQuality(val as TranscodeQuality);
        toast.success('Default quality updated.');
    };

    return (
        <div class="settings-panel-content general-panel">
            <h2 class="settings-panel-title">General</h2>

            <SectionGroup
                title="Performance"
                description="Configure background processing power. Higher values use more CPU but generate thumbnails faster."
            >
                <div class="general-setting-row">
                    <span class="setting-label">Thumbnail Threads:</span>
                    <div style={{ width: '200px' }}>
                        <Select
                            options={threadOptions}
                            value={threads()}
                            onValueChange={handleThreadChange}
                            placeholder="Select threads"
                        />
                    </div>
                </div>
                <p class="setting-note">* Requires restart to apply.</p>
            </SectionGroup>

            <SectionGroup
                title="Transcoding Cache"
                description="Manage cached video/audio files that were transcoded for playback."
            >
                <div class="cache-stats">
                    <div class="cache-stat-item">
                        <span class="cache-stat-label">Files:</span>
                        <span class="cache-stat-value">{cacheStats().file_count}</span>
                    </div>
                    <div class="cache-stat-item">
                        <span class="cache-stat-label">Size:</span>
                        <span class="cache-stat-value">{formatBytes(cacheStats().size_bytes)}</span>
                    </div>
                </div>
                <div class="general-setting-row">
                    <span class="setting-label">Auto-cleanup after:</span>
                    <div style={{ width: '140px' }}>
                        <Select
                            options={retentionOptions}
                            value={cacheRetentionDays()}
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
                                const val = parseInt(e.currentTarget.value);
                                if (!isNaN(val) && val > 0) {
                                    filterActions.setHistoryLimit(val);
                                }
                            }}
                        />
                    </div>
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
