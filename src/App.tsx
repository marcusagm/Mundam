import { onMount, onCleanup, Show, createEffect, createSignal } from 'solid-js';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { useSystem, useNotification } from './core/hooks';
import { AppShell } from './layouts/AppShell';
import { LibrarySidebar } from './components/layout/LibrarySidebar';
import { FileInspector } from './components/layout/FileInspector';
import { GlobalStatusbar } from './components/layout/GlobalStatusbar';
import { Viewport } from './components/layout/Viewport';
import { open } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
// Native DnD
import {
    dndRegistry,
    TagDropStrategy,
    ImageDropStrategy,
    currentDragItem,
    setDropTargetId
} from './core/dnd';
import { Sonner } from './components/ui/Sonner';
import { Loader } from './components/ui/Loader';
import { SettingsModal } from './components/features/settings';
// Input System
import { InputProvider, useShortcuts } from './core/input';
import { useSelection, useLibrary } from './core/hooks';

function App() {
    const system = useSystem();
    const notification = useNotification();
    const selection = useSelection();
    const lib = useLibrary();
    const [isSettingsOpen, setIsSettingsOpen] = createSignal(false);

    // Global shortcuts via Input Service
    useShortcuts([
        {
            keys: 'Meta+Comma',
            name: 'Settings',
            action: () => setIsSettingsOpen(true)
        },
        {
            keys: 'Meta+KeyA',
            name: 'Select All',
            action: () => {
                const allIds = lib.items.map(i => i.id);
                selection.select(allIds);
            }
        },
        {
            keys: 'Escape',
            name: 'Deselect All',
            action: () => {
                const active = document.activeElement;
                if (active && ['INPUT', 'TEXTAREA'].includes(active.tagName)) {
                    (active as HTMLElement).blur();
                } else {
                    selection.select([]);
                }
            }
        }
    ]);

    // Root-level DND cleanup
    createEffect(() => {
        if (!currentDragItem()) {
            setDropTargetId(null);
        }
    });

    onMount(() => {
        system.initialize();
        import('./core/store/appearanceStore').then(({ appearanceActions }) => {
            appearanceActions.initialize();
        });

        // Register Strategies
        dndRegistry.register('TAG', TagDropStrategy);
        dndRegistry.register('IMAGE', ImageDropStrategy);

        // Listen for indexing completion
        listen('indexer:complete', () => {
            notification.success('Indexing Complete', 'Library update finished');
        });

        // Notify Splash Screen
        window.dispatchEvent(new CustomEvent('app-ready'));

        // Listen for settings open requests
        const handleOpenSettings = () => setIsSettingsOpen(true);
        window.addEventListener('app:open-settings', handleOpenSettings);

        const handleOpenDesignSystem = async () => {
            try {
                const label = 'design-system';
                const existing = await WebviewWindow.getByLabel(label);
                if (existing) {
                    await existing.setFocus();
                    return;
                }

                const webview = new WebviewWindow(label, {
                    url: 'index.html#design-system',
                    title: 'Elleven Design System',
                    width: 1200,
                    height: 900
                });

                webview.once('tauri://error', function (e) {
                    console.error('webview error', e);
                });
            } catch (e) {
                console.error('Failed to open design system window', e);
            }
        };
        window.addEventListener('app:open-design-system', handleOpenDesignSystem);

        onCleanup(() => {
            window.removeEventListener('app:open-settings', handleOpenSettings);
            window.removeEventListener('app:open-design-system', handleOpenDesignSystem);
        });
    });

    const handleSelectFolder = async () => {
        try {
            const selected = await open({
                directory: true,
                multiple: false,
                title: 'Select Reference Library Folder'
            });

            if (selected) {
                const path = typeof selected === 'string' ? selected : (selected as any).path;
                if (path) {
                    notification.info(
                        'Indexing Started',
                        `Processing folder: ${path.split(/[\\/]/).pop()}`
                    );
                    await system.setRootLocation(path);
                }
            }
        } catch (err) {
            console.error('Failed to select folder:', err);
        }
    };

    return (
        <Show
            when={!system.loading()}
            fallback={<Loader fullscreen text="Initializing Elleven Library..." />}
        >
            <Show
                when={system.rootPath()}
                fallback={
                    <div class="welcome-screen">
                        <h1>Elleven Library</h1>
                        <p>Start by choosing a folder to monitor for visual references.</p>
                        <button class="primary-btn" onClick={handleSelectFolder}>
                            Initialize Library
                        </button>
                    </div>
                }
            >
                <AppShell
                    sidebar={<LibrarySidebar />}
                    inspector={<FileInspector />}
                    statusbar={<GlobalStatusbar />}
                >
                    <Viewport />
                </AppShell>
                <Sonner position="bottom-right" richColors />
                <SettingsModal
                    isOpen={isSettingsOpen()}
                    onClose={() => setIsSettingsOpen(false)}
                    initialTab="keyboard-shortcuts"
                />
            </Show>
        </Show>
    );
}

function AppWithProvider() {
    return (
        <InputProvider>
            <App />
        </InputProvider>
    );
}

export default AppWithProvider;
