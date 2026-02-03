import { createContext, useContext, createSignal, ParentComponent, Accessor, Setter } from "solid-js";

export interface FlipState {
    horizontal: boolean;
    vertical: boolean;
}

export interface Position {
    x: number;
    y: number;
}

export type ViewportTool = "pan" | "rotate";

export type MediaType = 'image' | 'video' | 'audio' | 'font' | 'model' | 'unknown';

interface ViewportContextState {
    zoom: Accessor<number>;
    setZoom: Setter<number>;
    rotation: Accessor<number>;
    setRotation: Setter<number>;
    flip: Accessor<FlipState>;
    setFlip: Setter<FlipState>;
    tool: Accessor<ViewportTool>;
    setTool: Setter<ViewportTool>;
    position: Accessor<Position>;
    setPosition: Setter<Position>;
    mediaType: Accessor<MediaType>;
    setMediaType: Setter<MediaType>;
    reset: () => void;
}

const ViewportContext = createContext<ViewportContextState>();

export const ViewportProvider: ParentComponent = (props) => {
    // Zoom agora armazena a porcentagem real (ex: 100 = 100%).
    // O valor inicial pode ser sobrescrito pelo renderizador (ex: ImageViewer setando Fit on load).
    const [zoom, setZoom] = createSignal(100); 
    const [rotation, setRotation] = createSignal(0);
    const [flip, setFlip] = createSignal<FlipState>({ horizontal: false, vertical: false });
    const [tool, setTool] = createSignal<ViewportTool>("pan");
    const [position, setPosition] = createSignal<Position>({ x: 0, y: 0 });
    const [mediaType, setMediaType] = createSignal<MediaType>('unknown');

    const reset = () => {
        setZoom(100);
        setRotation(0);
        setFlip({ horizontal: false, vertical: false });
        setPosition({ x: 0, y: 0 });
        setTool("pan");
        setMediaType('unknown');
    };

    const value: ViewportContextState = {
        zoom, setZoom,
        rotation, setRotation,
        flip, setFlip,
        tool, setTool,
        position, setPosition,
        mediaType, setMediaType,
        reset
    };

    return (
        <ViewportContext.Provider value={value}>
            {props.children}
        </ViewportContext.Provider>
    );
};

export const useViewportContext = () => {
    const context = useContext(ViewportContext);
    if (!context) {
        throw new Error("useViewportContext must be used within a ViewportProvider");
    }
    return context;
};
