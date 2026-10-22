import { createStore } from 'solid-js/store';
import { LayoutConfig, LayoutItemInput, ItemPosition, LayoutMode } from '../viewport/types';
import { selectionActions } from './selectionStore';

/**
 * Viewport State
 *
 * Centralized state for the library virtualization and navigation.
 */
interface ViewportState {
    /** Current layout mode */
    layoutMode: LayoutMode;
    /** Current layout configuration */
    config: LayoutConfig;
    /** Total items loaded in the current view */
    items: LayoutItemInput[];
    /** Total scrollable height calculated by worker */
    totalHeight: number;
    /** Whether a layout calculation is in progress */
    isCalculating: boolean;
    /** Currently visible items in the viewport */
    visibleItems: ItemPosition[];
    /** Last known scroll position */
    scrollPosition: {
        scrollTop: number;
        viewportHeight: number;
    };
    /** Currently focused item ID (e.g., in ItemView) */
    focusedItemId: string | null;
    /** Asset the list viewport must scroll into view once it is laid out */
    scrollTargetId: string | null;
    /** Zoom level for the immersive viewer (percentage, default 100) */
    zoom: number;
    /** Fit to screen mapping */
    fitToScreen: boolean;
    /** Panning offsets */
    pan: { x: number; y: number };
}

const DEFAULT_CONFIG: LayoutConfig = {
    mode: 'masonry',
    containerWidth: 0,
    itemSize: 280,
    gap: 16,
    buffer: 1000
};

const [viewportState, setViewportState] = createStore<ViewportState>({
    layoutMode: 'masonry',
    config: DEFAULT_CONFIG,
    items: [],
    totalHeight: 0,
    isCalculating: false,
    visibleItems: [],
    scrollPosition: {
        scrollTop: 0,
        viewportHeight: 800
    },
    focusedItemId: null,
    scrollTargetId: null,
    zoom: 100,
    fitToScreen: true,
    pan: { x: 0, y: 0 }
});

export const viewportActions = {
    /** Requests the gallery viewport to scroll the given asset into view. */
    requestScrollToAsset: (id: string | null) => {
        setViewportState('scrollTargetId', id);
    },

    /**
     * Updates the focused item and synchronizes library selection.
     */
    setFocusedItem: (id: string | null) => {
        setViewportState('focusedItemId', id);
        if (id !== null) {
            // Sincroniza seleção global: seleciona apenas o item atual
            // para que a library reflita o que está sendo visualizado.
            selectionActions.select([id]);
        }
    },

    /**
     * Navigates to the next or previous asset in the current list.
     */
    navigateToAsset: (direction: 'next' | 'prev') => {
        const currentId = viewportState.focusedItemId;
        if (currentId === null || viewportState.items.length === 0) return;

        const currentIndex = viewportState.items.findIndex(
            (item: LayoutItemInput) => item.id === currentId
        );
        if (currentIndex === -1) return;

        let nextIndex = direction === 'next' ? currentIndex + 1 : currentIndex - 1;

        // Wrap around or clamp? usually clamp for professional viewers.
        if (nextIndex < 0) nextIndex = 0;
        if (nextIndex >= viewportState.items.length) nextIndex = viewportState.items.length - 1;

        const nextId = viewportState.items[nextIndex].id;
        viewportActions.setFocusedItem(nextId);
    },

    /**
     * Updates internal state from worker results.
     * Internal use only.
     */
    updateFromWorker: (data: {
        totalHeight?: number;
        isCalculating?: boolean;
        visibleItems?: ItemPosition[];
    }) => {
        if (data.totalHeight !== undefined) setViewportState('totalHeight', data.totalHeight);
        if (data.isCalculating !== undefined) setViewportState('isCalculating', data.isCalculating);
        if (data.visibleItems !== undefined) setViewportState('visibleItems', data.visibleItems);
    },

    /**
     * Sets items and resets focus if necessary.
     */
    setItems: (items: LayoutItemInput[]) => {
        setViewportState('items', items);
    },

    /**
     * Updates layout configuration.
     */
    setConfig: (config: Partial<LayoutConfig>) => {
        setViewportState('config', (current: LayoutConfig) => ({ ...current, ...config }));
        if (config.mode) setViewportState('layoutMode', config.mode);
    },

    /**
     * Updates scroll position.
     */
    setScroll: (scrollTop: number, viewportHeight: number) => {
        setViewportState('scrollPosition', { scrollTop, viewportHeight });
    },

    // ============================================================================
    // Immersive Viewer Actions (Zoom / Pan / Fit)
    // ============================================================================

    /**
     * Sets the zoom level, optionally overriding fitToScreen.
     */
    setZoom: (zoom: number) => {
        setViewportState('zoom', Math.max(5, Math.min(500, zoom)));
        setViewportState('fitToScreen', false);
    },

    /**
     * Toggles or sets the fit-to-screen state.
     */
    setFitToScreen: (fit?: boolean) => {
        const value = fit ?? !viewportState.fitToScreen;
        setViewportState('fitToScreen', value);
        if (value) {
            setViewportState('pan', { x: 0, y: 0 }); // reset pan when fitting
        }
    },

    /**
     * Updates the pan offset.
     */
    setPan: (x: number, y: number) => {
        setViewportState('pan', { x, y });
    },

    /**
     * Resets pan and zoom to default.
     */
    resetView: () => {
        setViewportState('zoom', 100);
        setViewportState('fitToScreen', true);
        setViewportState('pan', { x: 0, y: 0 });
    }
};

export { viewportState };
