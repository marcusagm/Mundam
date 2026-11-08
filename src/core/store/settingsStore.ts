import { createSignal } from 'solid-js';
import { tauriService } from '../tauri/services';
import { ActionResult, ErrorCode } from '../types/actions';
import { SettingsPayloadSchema, type SettingsPayload } from './settings/schemas';

export interface CacheStats {
    size_bytes: number;
    file_count: number;
}

const [processingConcurrency, setProcessingConcurrency] = createSignal<number>(0);
const [thumbnailThreads, setThumbnailThreads] = createSignal<number>(2);
const [cacheRetentionDays, setCacheRetentionDays] = createSignal<number>(30);
const [indexerConcurrencyLimit, setIndexerConcurrencyLimit] = createSignal<number>(200);
const [autoDetectDuplicates, setAutoDetectDuplicates] = createSignal<boolean>(true);
const [developerMode, setDeveloperMode] = createSignal<boolean>(false);
const [cacheStats, setCacheStats] = createSignal<CacheStats>({
    size_bytes: 0,
    file_count: 0
});

export const settingsActions = {
    initialize: async () => {
        const [threads, retention, concurrency, autoDetect, stats, budget, developerModeSetting] =
            await Promise.all([
                tauriService.getSetting('thumbnail_threads'),
                tauriService.getSetting('cache_retention_days'),
                tauriService.getSetting('indexer_concurrency_limit'),
                tauriService.getSetting('auto_detect_duplicates'),
                tauriService.getCacheStats(),
                tauriService.getSetting('processing_concurrency'),
                tauriService.getSetting('developer_mode')
            ]);

        if (budget !== null) setProcessingConcurrency(Number(budget));
        if (threads !== null) setThumbnailThreads(Number(threads));
        if (retention !== null) setCacheRetentionDays(Number(retention));
        if (concurrency !== null) setIndexerConcurrencyLimit(Number(concurrency));
        if (autoDetect !== null) setAutoDetectDuplicates(Boolean(autoDetect));
        if (developerModeSetting !== null) {
            setDeveloperMode(developerModeSetting === 'true' || developerModeSetting === '1');
        }
        setCacheStats({ size_bytes: stats.size_bytes, file_count: stats.file_count });
    },

    updateSettings: async (payload: SettingsPayload): Promise<ActionResult> => {
        const validation = SettingsPayloadSchema.safeParse(payload);
        if (!validation.success) {
            return {
                success: false,
                error: {
                    code: ErrorCode.VALIDATION_ERROR,
                    message: 'Invalid settings payload'
                }
            };
        }

        const promises: Promise<void>[] = [];
        if (payload.processingConcurrency !== undefined) {
            const value = payload.processingConcurrency;
            promises.push(
                tauriService.setSetting('processing_concurrency', String(value)).then(() => {
                    setProcessingConcurrency(value);
                })
            );
        }
        if (payload.thumbnailThreads !== undefined) {
            const value = payload.thumbnailThreads;
            promises.push(
                tauriService.setSetting('thumbnail_threads', String(value)).then(() => {
                    setThumbnailThreads(value);
                })
            );
        }
        if (payload.cacheRetentionDays !== undefined) {
            const value = payload.cacheRetentionDays;
            promises.push(
                tauriService.setSetting('cache_retention_days', String(value)).then(() => {
                    setCacheRetentionDays(value);
                })
            );
        }
        if (payload.indexerConcurrencyLimit !== undefined) {
            const value = payload.indexerConcurrencyLimit;
            promises.push(
                tauriService.setSetting('indexer_concurrency_limit', String(value)).then(() => {
                    setIndexerConcurrencyLimit(value);
                })
            );
        }
        if (payload.autoDetectDuplicates !== undefined) {
            const value = payload.autoDetectDuplicates;
            promises.push(
                tauriService.setSetting('auto_detect_duplicates', String(value)).then(() => {
                    setAutoDetectDuplicates(value);
                })
            );
        }
        if (payload.developerMode !== undefined) {
            const developerModeValue = payload.developerMode;
            promises.push(
                tauriService.setSetting('developer_mode', String(developerModeValue)).then(() => {
                    setDeveloperMode(developerModeValue);
                })
            );
        }

        try {
            await Promise.all(promises);
            return { success: true, data: undefined };
        } catch (error) {
            console.error('Failed to save settings:', error);
            return {
                success: false,
                error: {
                    code: ErrorCode.IO_ERROR,
                    message: 'Failed to save settings to disk'
                }
            };
        }
    },

    refreshCacheStats: async () => {
        const stats = await tauriService.getCacheStats();
        setCacheStats({ size_bytes: stats.size_bytes, file_count: stats.file_count });
    }
};

export {
    processingConcurrency,
    thumbnailThreads,
    cacheRetentionDays,
    indexerConcurrencyLimit,
    autoDetectDuplicates,
    developerMode,
    cacheStats
};
