import { Component, JSX, createSignal, createContext, useContext, Accessor } from 'solid-js';
import { useCommands } from '../core/input';
import { ResizablePanelGroup, ResizablePanel, ResizableHandle } from '../components/ui/Resizable';
import '../styles/global.css';
import './app-shell.css';

/**
 * Context value for the AppShell to control its various sections.
 */
interface AppShellContextValue {
    /** Whether the left library sidebar is currently visible */
    isSidebarOpen: Accessor<boolean>;
    /** Toggles the visibility of the left sidebar */
    toggleSidebar: () => void;
    /** Whether the right property inspector is currently visible */
    isInspectorOpen: Accessor<boolean>;
    /** Toggles the visibility of the right inspector */
    toggleInspector: () => void;
}

/** Context for sharing AppShell control state with sub-components (like Statusbar) */
const AppShellContext = createContext<AppShellContextValue>();

/**
 * Hook to access the AppShell context.
 *
 * @returns {AppShellContextValue | undefined} The shell state and actions.
 */
export const useAppShell = () => useContext(AppShellContext);

/**
 * Properties for the AppShell component.
 */
interface AppShellProperties {
    /** The main content area children */
    children: JSX.Element;
    /** Optional header element */
    header?: JSX.Element;
    /** Optional left sidebar content (e.g., LibrarySidebar) */
    sidebar?: JSX.Element;
    /** Optional right inspector content */
    inspector?: JSX.Element;
    /** Optional bottom statusbar content */
    statusbar?: JSX.Element;
}

/**
 * The core layout system of the application.
 * Implements a 3-pane structure with resizable horizontal areas and persistence.
 *
 * Layout Structure:
 * [ Header Area (Optional) ]
 * [ Sidebar (Resizable) | Content (Flexible) | Inspector (Resizable) ]
 * [ Statusbar (Fixed) ]
 *
 * @param {AppShellProperties} props - Component properties.
 * @returns {JSX.Element} The rendered shell.
 */
export const AppShell: Component<AppShellProperties> = props => {
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
            {props.header && <header class="shell-header">{props.header}</header>}

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
                    {props.sidebar}
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
                    {props.children}
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
                    {props.inspector}
                </ResizablePanel>
            </ResizablePanelGroup>

            {/* Bottom Global Statusbar Area */}
            <footer class="shell-footer">
                <AppShellContext.Provider
                    value={{
                        isSidebarOpen,
                        toggleSidebar,
                        isInspectorOpen,
                        toggleInspector
                    }}
                >
                    {props.statusbar}
                </AppShellContext.Provider>
            </footer>
        </div>
    );
};
