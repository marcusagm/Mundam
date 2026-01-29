import { createSignal, createRoot } from "solid-js";

export type ViewportMode = "list" | "item";

function createViewportState() {
    const [mode, setMode] = createSignal<ViewportMode>("list");
    const [activeItemId, setActiveItemId] = createSignal<string | null>(null);
    const [history, setHistory] = createSignal<string[]>([]);
    const [historyIndex, setHistoryIndex] = createSignal(-1);

    const openItem = (id: string) => {
        setActiveItemId(id);
        setMode("item");
        
        // Simple history tracking
        const newHistory = history().slice(0, historyIndex() + 1);
        newHistory.push(id);
        setHistory(newHistory);
        setHistoryIndex(newHistory.length - 1);
    };

    const closeItem = () => {
        setMode("list");
        setActiveItemId(null);
    };

    const goBack = () => {
        if (historyIndex() > 0) {
            setHistoryIndex(prev => prev - 1);
            setActiveItemId(history()[historyIndex()]);
        } else if (mode() === "item") {
            closeItem();
        }
    };

    const goForward = () => {
        if (historyIndex() < history().length - 1) {
            setHistoryIndex(prev => prev + 1);
            setActiveItemId(history()[historyIndex()]);
            setMode("item");
        }
    };

    return {
        mode,
        activeItemId,
        openItem,
        closeItem,
        goBack,
        goForward,
        canGoBack: () => historyIndex() > 0 || mode() === "item",
        canGoForward: () => historyIndex() < history().length - 1
    };
}

const viewportState = createRoot(createViewportState);

export const useViewport = () => viewportState;
