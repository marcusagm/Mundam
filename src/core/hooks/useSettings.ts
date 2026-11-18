import {
    processingConcurrency,
    linuxVideoCompatibility,
    linuxGraphicsCompatibility,
    thumbnailThreads,
    cacheRetentionDays,
    indexerConcurrencyLimit,
    autoDetectDuplicates,
    developerMode,
    cacheStats,
    settingsActions
} from '../store/settingsStore';

/**
 * Hook providing access to application settings and management actions.
 * Centralizes state access for settings panels.
 *
 * @returns {Object} Settings state and actions.
 */
export const useSettings = () => {
    return {
        /** Shared media operations; zero selects automatic capacity. */
        processingConcurrency,
        /** Saved Linux video compatibility mode, applied after restart. */
        linuxVideoCompatibility,
        /** Saved Linux interface compatibility mode, applied before splash after restart. */
        linuxGraphicsCompatibility,
        /** Number of worker threads for thumbnail generation */
        thumbnailThreads,
        /** Number of days to keep transcoded files in cache */
        cacheRetentionDays,
        /** Maximum concurrent tasks during indexing */
        indexerConcurrencyLimit,
        /** Whether automatic duplicate detection is triggered upon asset import */
        autoDetectDuplicates,
        /** Whether developer diagnostics and telemetry are enabled in statusbar and views */
        developerMode,
        /** Current cache usage statistics */
        cacheStats,

        /** Initialize settings from backend */
        initialize: settingsActions.initialize,
        /** Update one or more settings with validation */
        updateSettings: settingsActions.updateSettings,
        /** Force refresh of cache statistics */
        refreshCacheStats: settingsActions.refreshCacheStats
    };
};
