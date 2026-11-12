import { invokeCommand as invoke } from '../../lib/api';
import { type FileFormat } from '../store/formatStore';
import { LifecycleManager } from '../utils/LifecycleManager';

// Define strict types for Tauri commands
export interface StartIndexingArgs {
    path: string;
}

export const tauriService = {
    /**
     * Starts the background indexing process for the given directory path.
     * Reports progress and either 'indexer:complete' or 'indexer:failed'.
     */
    startIndexing: async (args: StartIndexingArgs): Promise<void> => {
        try {
            await invoke('start_indexing', { path: args.path });
            LifecycleManager.logTelemetry(
                'info',
                'tauriService',
                `Started indexing for: ${args.path}`
            );
        } catch (error) {
            LifecycleManager.logTelemetry(
                'error',
                'tauriService',
                `Command 'start_indexing' failed: ${String(error)}`
            );
            throw error;
        }
    },

    /**
     * Example wrapper for other commands...
     */
    // stopIndexing: async () => invoke("stop_indexing"),

    getLibrarySupportedFormats: async (): Promise<FileFormat[]> => {
        try {
            return await invoke('get_library_supported_formats');
        } catch (error) {
            LifecycleManager.logTelemetry(
                'error',
                'tauriService',
                `Failed to load formats: ${String(error)}`
            );
            return [];
        }
    },

    runDbMaintenance: async (): Promise<void> => {
        try {
            await invoke('run_db_maintenance');
            LifecycleManager.logTelemetry(
                'info',
                'tauriService',
                'DB Maintenance completed successfully'
            );
        } catch (error) {
            LifecycleManager.logTelemetry(
                'error',
                'tauriService',
                `Failed to run DB maintenance: ${String(error)}`
            );
            throw error;
        }
    },

    getSetting: async (key: string): Promise<string | null> => {
        try {
            return await invoke('get_setting', { key });
        } catch (error) {
            LifecycleManager.logTelemetry(
                'error',
                'tauriService',
                `Failed to get setting ${key}: ${String(error)}`
            );
            return null;
        }
    },

    setSetting: async (key: string, value: string): Promise<void> => {
        try {
            await invoke('set_setting', { key, value });
        } catch (error) {
            LifecycleManager.logTelemetry(
                'error',
                'tauriService',
                `Failed to set setting ${key}: ${String(error)}`
            );
            throw error;
        }
    },

    // --- Cache Management ---

    getCacheStats: async (): Promise<{
        directory: string;
        size_bytes: number;
        file_count: number;
    }> => {
        try {
            // V2 backend exposes this command as get_library_cache_stats
            const response = await invoke<{
                thumbnails: { count: number; size: number };
                hls: { count: number; size: number };
                total: { count: number; size: number };
            }>('get_library_cache_stats');
            return {
                directory: '',
                size_bytes: response.total.size,
                file_count: response.total.count
            };
        } catch (error) {
            LifecycleManager.logTelemetry(
                'error',
                'tauriService',
                `Failed to get cache stats: ${String(error)}`
            );
            return { directory: '', size_bytes: 0, file_count: 0 };
        }
    },

    // cleanupCache: async (_maxAgeDays?: number): Promise<number> => {
    cleanupCache: async (): Promise<number> => {
        try {
            // V2 backend cleanup_cache clears HLS cache and returns void.
            // We capture the count before and after to compute deleted entries.
            const statsBefore = await tauriService.getCacheStats();
            await invoke('cleanup_cache');
            const statsAfter = await tauriService.getCacheStats();
            const deletedCount = Math.max(0, statsBefore.file_count - statsAfter.file_count);
            LifecycleManager.logTelemetry(
                'info',
                'tauriService',
                `Cache cleaned. Cleared ${deletedCount} entries.`
            );
            return deletedCount;
        } catch (error) {
            LifecycleManager.logTelemetry(
                'error',
                'tauriService',
                `Failed to cleanup cache: ${String(error)}`
            );
            throw error;
        }
    },

    clearCache: async (): Promise<number> => {
        try {
            // V2 backend clear_cache clears both thumbnails and HLS and returns void.
            const statsBefore = await tauriService.getCacheStats();
            await invoke('clear_cache');
            LifecycleManager.logTelemetry(
                'info',
                'tauriService',
                `Cache entirely cleared. Removed ${statsBefore.file_count} entries.`
            );
            return statsBefore.file_count;
        } catch (error) {
            LifecycleManager.logTelemetry(
                'error',
                'tauriService',
                `Failed to clear cache: ${String(error)}`
            );
            throw error;
        }
    }
};
