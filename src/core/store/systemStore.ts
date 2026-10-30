import { createSignal } from 'solid-js';
import { type FileFormat, formatState } from './formatStore';
import { listen } from '@tauri-apps/api/event';
import { addLocation, initDb } from '../../lib/db';
import { tauriService } from '../tauri/services';
import { metadataActions, metadataState } from './metadata';
import { libraryActions, type BatchChangePayload } from './library';
import { initStreamingToken } from '../../lib/hls-player';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

export interface ProgressPayload {
    total: number;
    processed: number;
    current_file: string;
}

const [loading, setLoading] = createSignal(true);
const [progress, setProgress] = createSignal<ProgressPayload | null>(null);
const [thumbnailProgress, setThumbnailProgress] = createSignal<ProgressPayload | null>(null);
const [rootPath, setRootPath] = createSignal<string | null>(null);
const [initialized, setInitialized] = createSignal(false);
const [supportedFormats, setSupportedFormats] = createSignal<FileFormat[]>([]);
const [isSettingsOpen, setIsSettingsOpen] = createSignal(false);
const [isDesignSystemOpen, setIsDesignSystemOpen] = createSignal(false);

export const systemActions = {
    initialize: async () => {
        if (initialized()) return;

        try {
            setLoading(true);
            await initDb();
            await initStreamingToken();
            await metadataActions.loadLocations(true);
            await metadataActions.loadTags(true);
            await metadataActions.loadSmartFolders(true);

            setSupportedFormats(formatState.formats);

            // Auto-select root path if locations exist
            if (metadataState.locations.length > 0) {
                const main = metadataState.locations[0];
                setRootPath(main.path);
                // Trigger initial load
                await libraryActions.refreshAssets(true);
                metadataActions.loadStats();
            }

            // Setup Listeners
            await listen<ProgressPayload>('indexer:progress', e => {
                systemActions.updateProgress(e.payload);
            });

            await listen<number>('indexer:complete', () => {
                systemActions.clearProgress();
                libraryActions.refreshAssets(true);
                metadataActions.loadStats();
                metadataActions.loadLocations();
            });

            await listen<{ id: string; path: string }>('thumbnail:ready', e => {
                libraryActions.updateThumbnail(e.payload.id, e.payload.path);
            });

            await listen<{ id: string }>('metadata:ready', e => {
                libraryActions.refreshItem(e.payload.id);
            });

            await listen<ProgressPayload>('thumbnail:queue-status', e => {
                systemActions.updateThumbnailProgress(e.payload);
            });

            await listen<BatchChangePayload>('library:batch-change', e => {
                const payload = e.payload;
                libraryActions.handleBatchChange(payload);
                metadataActions.handleBatchChange(payload);
            });

            setInitialized(true);
            setLoading(false);
        } catch (err) {
            console.error('Initialization failed:', err);
            throw err;
        }
    },

    setRootLocation: async (path: string) => {
        await addLocation(path);
        await metadataActions.loadLocations();
        setRootPath(path);
        await tauriService.startIndexing({ path });
    },

    updateProgress: (payload: ProgressPayload) => {
        setProgress(payload);
    },

    clearProgress: () => {
        setProgress(null);
    },

    setRootPath: (path: string | null) => {
        setRootPath(path);
    },

    setLoading: (isLoading: boolean) => {
        setLoading(isLoading);
    },

    updateThumbnailProgress: (payload: ProgressPayload) => {
        setThumbnailProgress(payload);
    },

    openSettings: (open = true) => {
        setIsSettingsOpen(open);
    },

    openDesignSystem: async (open = true) => {
        if (!open) {
            setIsDesignSystemOpen(false);
            return;
        }

        try {
            const label = 'design-system';
            const existing = await WebviewWindow.getByLabel(label);
            if (existing) {
                await existing.setFocus();
                return;
            }

            const webview = new WebviewWindow(label, {
                url: 'index.html#design-system',
                title: 'Mundam Design System',
                width: 1200,
                height: 900
            });

            webview.once('tauri://error', error => {
                console.error('Webview error:', error);
            });

            setIsDesignSystemOpen(true);
        } catch (error) {
            console.error('Failed to open design system window:', error);
        }
    },

    runDbMaintenance: async () => {
        await tauriService.runDbMaintenance();
    },

    cleanupCache: async () => {
        return await tauriService.cleanupCache();
    },

    clearCache: async () => {
        return await tauriService.clearCache();
    }
};

export {
    loading,
    progress,
    thumbnailProgress,
    rootPath,
    supportedFormats,
    isSettingsOpen,
    isDesignSystemOpen
};
