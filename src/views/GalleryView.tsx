import { Component, JSX, createEffect, createMemo, createSignal } from 'solid-js';
import { useCommands } from '../core/input';
import { useLibrary, useSelection } from '../core/hooks';
import { ResizablePanelGroup, ResizablePanel, ResizableHandle } from '../components/ui/Resizable';
import './gallery-view.css';
import { StatusCounts } from '../components/features/statusbar/StatusCounts';
import { Button } from '../components/ui';
import { PanelLeftClose, PanelLeftOpen, PanelRightClose, PanelRightOpen } from 'lucide-solid';
import { LibrarySidebar } from '../components/layout/LibrarySidebar';
import { FileInspector } from '../components/layout/FileInspector';
import { GlobalStatusbar } from '../components/layout/GlobalStatusbar';
import { Viewport } from '../components/layout/Viewport';
import {
    pluginInspectorSectionKey,
    pluginNavigationLabels,
    type PluginInspectorDataClient,
    type PluginNavigationSurface
} from '../lib/pluginInspector';
import type { PluginPresentationState } from '../core/hooks/createPluginPresentation';
import { PluginSectionActionMenu } from '../components/features/inspector/plugins/PluginSectionActionMenu';

export interface GalleryViewProperties {
    /** Renders the shell header with the gallery's optional actions. */
    header: (pluginActions: JSX.Element) => JSX.Element;
    /** Supplies approved plugin sections independently of the desktop transport. */
    pluginPresentation: PluginPresentationState;
    pluginInspectorData: PluginInspectorDataClient;
    focusOnReturn?: boolean;
    onReturnFocused?: () => void;
}

/** Renders the gallery and persists its resizable panels. */
export const GalleryView: Component<GalleryViewProperties> = props => {
    /** LocalStorage key for persisting panel percentages */
    const STORAGE_KEY_LAYOUT = 'app-shell-layout';
    /** LocalStorage key for persisting collapsed/expanded states */
    const STORAGE_KEY_STATES = 'app-shell-states';

    /**
     * Retrieves the persisted layout from local storage.
     *
     * @returns {number[] | null} Array of sizes or null.
     */
    const getPersistedLayout = (): number[] | null => {
        try {
            const savedLayout = localStorage.getItem(STORAGE_KEY_LAYOUT);
            return savedLayout ? JSON.parse(savedLayout) : null;
        } catch {
            return null;
        }
    };

    /**
     * Retrieves the persisted open/collapsed states from local storage.
     *
     * @returns {{ sidebar: boolean; inspector: boolean }} States object.
     */
    const getPersistedStates = (): { sidebar: boolean; inspector: boolean } => {
        try {
            const savedStates = localStorage.getItem(STORAGE_KEY_STATES);
            return savedStates ? JSON.parse(savedStates) : { sidebar: true, inspector: true };
        } catch {
            return { sidebar: true, inspector: true };
        }
    };

    const persistedLayout = getPersistedLayout();
    const persistedStates = getPersistedStates();

    const [isSidebarOpen, setIsSidebarOpen] = createSignal(persistedStates.sidebar);
    const [isInspectorOpen, setIsInspectorOpen] = createSignal(persistedStates.inspector);

    const library = useLibrary();
    const selection = useSelection();
    const eligibleAssetId = createMemo(() => {
        const [assetId] = selection.selectedIds;
        return selection.selectedIds.length === 1 &&
            library.items.some(item => item.id === assetId && !item.deleted_at)
            ? assetId
            : undefined;
    });
    const sidebarPluginSections = () =>
        isSidebarOpen() ? props.pluginPresentation.sections() : undefined;
    const selectionPluginSections = () =>
        eligibleAssetId() ? props.pluginPresentation.sections() : undefined;
    const [revealRequest, setRevealRequest] = createSignal<{
        assetId: string;
        key: string;
        surface: PluginNavigationSurface;
    }>();
    const canRevealSection = (key: string, surface: PluginNavigationSurface) =>
        props.pluginPresentation
            .sections()
            ?.some(
                section =>
                    pluginInspectorSectionKey(section) === key &&
                    section[pluginNavigationLabels[surface]]
            );
    const requestedSection = createMemo(() => {
        const request = revealRequest();
        return request &&
            request.assetId === eligibleAssetId() &&
            canRevealSection(request.key, request.surface)
            ? request.key
            : undefined;
    });
    createEffect(() => {
        if (!requestedSection()) setRevealRequest(undefined);
    });

    /**
     * Persists the visibility states to local storage.
     */
    const saveStates = (sidebarVisible: boolean, inspectorVisible: boolean) => {
        localStorage.setItem(
            STORAGE_KEY_STATES,
            JSON.stringify({
                sidebar: sidebarVisible,
                inspector: inspectorVisible
            })
        );
    };

    /**
     * Toggles the sidebar visibility.
     */
    const toggleSidebar = () => {
        setIsSidebarOpen(previousState => {
            const nextState = !previousState;
            saveStates(nextState, isInspectorOpen());
            return nextState;
        });
    };

    /**
     * Toggles the inspector visibility.
     */
    const toggleInspector = () => {
        setIsInspectorOpen(previousState => {
            const nextState = !previousState;
            saveStates(isSidebarOpen(), nextState);
            return nextState;
        });
    };

    const revealPluginSection = (key: string, surface: PluginNavigationSurface) => {
        const assetId = eligibleAssetId();
        if (!assetId || !canRevealSection(key, surface)) return;
        if (!isInspectorOpen()) {
            setIsInspectorOpen(true);
            saveStates(isSidebarOpen(), true);
        }
        setRevealRequest({ assetId, key, surface });
    };

    const toggleSidebars = () => {
        const shouldOpen = !isSidebarOpen() && !isInspectorOpen();
        setIsSidebarOpen(shouldOpen);
        setIsInspectorOpen(shouldOpen);
        saveStates(shouldOpen, shouldOpen);
    };

    useCommands({
        'app:toggle-sidebars': toggleSidebars
    });

    // Calculate initial sizes based on persistence or defaults
    const sidebarInitialSize = persistedLayout?.[0] ?? 18;
    const contentInitialSize = persistedLayout?.[1] ?? 62;
    const inspectorInitialSize = persistedLayout?.[2] ?? 20;

    /**
     * Handles updates to the layout percentages.
     */
    const handleLayoutChange = (newSizes: number[]) => {
        localStorage.setItem(STORAGE_KEY_LAYOUT, JSON.stringify(newSizes));
    };

    return (
        <div class="app-shell">
            {/* Title Bar / Header */}
            <header class="shell-header">
                {props.header(
                    <PluginSectionActionMenu
                        surface="titleBar"
                        buttonLabel="Title bar plugin actions"
                        sections={selectionPluginSections()}
                        onRevealSection={key => revealPluginSection(key, 'titleBar')}
                    />
                )}
            </header>

            <ResizablePanelGroup
                direction="horizontal"
                class="shell-body"
                onLayout={handleLayoutChange}
            >
                {/* Left Sidebar Pane */}
                <ResizablePanel
                    id="shell-sidebar"
                    defaultSize={sidebarInitialSize}
                    minSize={12}
                    maxSize={35}
                    class="shell-sidebar"
                    isCollapsed={!isSidebarOpen()}
                >
                    <LibrarySidebar
                        pluginSections={sidebarPluginSections()}
                        canOpenPluginLinks={eligibleAssetId() !== undefined}
                        onOpenPluginSection={key => revealPluginSection(key, 'sidebar')}
                    />
                </ResizablePanel>

                <ResizableHandle isCollapsed={!isSidebarOpen()} />

                {/* Central Application Viewport */}
                <ResizablePanel
                    id="shell-content"
                    defaultSize={contentInitialSize}
                    minSize={30}
                    flexGrow={1}
                    class="shell-content"
                >
                    <Viewport
                        pluginSections={selectionPluginSections()}
                        onRevealPluginSection={key => revealPluginSection(key, 'viewport')}
                        focusOnReturn={props.focusOnReturn}
                        onReturnFocused={props.onReturnFocused}
                    />
                </ResizablePanel>

                <ResizableHandle isCollapsed={!isInspectorOpen()} />

                {/* Right Inspector Pane */}
                <ResizablePanel
                    id="shell-inspector"
                    defaultSize={inspectorInitialSize}
                    minSize={22}
                    maxSize={40}
                    class="shell-inspector"
                    isCollapsed={!isInspectorOpen()}
                >
                    <FileInspector
                        pluginInspector={props.pluginPresentation}
                        pluginInspectorData={props.pluginInspectorData}
                        inspectorOpen={isInspectorOpen()}
                        revealSection={requestedSection()}
                        onSectionRevealed={() => setRevealRequest(undefined)}
                    />
                </ResizablePanel>
            </ResizablePanelGroup>

            <GlobalStatusbar
                context={
                    <>
                        <Button
                            variant="ghost"
                            size="icon-sm"
                            class="status-btn"
                            onClick={toggleSidebar}
                            aria-expanded={isSidebarOpen()}
                            aria-label={isSidebarOpen() ? 'Close Sidebar' : 'Open Sidebar'}
                            title={isSidebarOpen() ? 'Close Sidebar' : 'Open Sidebar'}
                        >
                            {isSidebarOpen() ? (
                                <PanelLeftClose size={12} />
                            ) : (
                                <PanelLeftOpen size={12} />
                            )}
                        </Button>
                        <StatusCounts />
                    </>
                }
                actions={
                    <Button
                        variant="ghost"
                        size="icon-sm"
                        class="status-btn"
                        onClick={toggleInspector}
                        aria-expanded={isInspectorOpen()}
                        aria-label={isInspectorOpen() ? 'Close Inspector' : 'Open Inspector'}
                        title={isInspectorOpen() ? 'Close Inspector' : 'Open Inspector'}
                    >
                        {isInspectorOpen() ? (
                            <PanelRightClose size={12} />
                        ) : (
                            <PanelRightOpen size={12} />
                        )}
                    </Button>
                }
            />
        </div>
    );
};
