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

interface ItemViewContextState {
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
    // Slideshow
    slideshowPlaying: Accessor<boolean>;
    setSlideshowPlaying: Setter<boolean>;
    slideshowDuration: Accessor<number>;
    setSlideshowDuration: Setter<number>;

    // Font Settings
    fontSettings: Accessor<FontSettings>;
    setFontSettings: Setter<FontSettings>;

    reset: () => void;
}

export interface FontSettings {
    color: string;
    backgroundColor: string;
    fontSize: number; // PX
    fontWeight: string;
    lineHeight: number;
    letterSpacing: number;
    variableAxes?: Record<string, number>;
}

const ItemViewContext = createContext<ItemViewContextState>();

export const ItemViewProvider: ParentComponent = (props) => {
    // Zoom agora armazena a porcentagem real (ex: 100 = 100%).
    // O valor inicial pode ser sobrescrito pelo renderizador (ex: ImageViewer setando Fit on load).
    const [zoom, setZoom] = createSignal(100); 
    const [rotation, setRotation] = createSignal(0);
    const [flip, setFlip] = createSignal<FlipState>({ horizontal: false, vertical: false });
    const [tool, setTool] = createSignal<ViewportTool>("pan");
    const [position, setPosition] = createSignal<Position>({ x: 0, y: 0 });
    const [mediaType, setMediaType] = createSignal<MediaType>('unknown');
    
    // Timer / Slideshow
    const [slideshowPlaying, setSlideshowPlaying] = createSignal(false);
    const [slideshowDuration, setSlideshowDuration] = createSignal(5); // seconds

    // Font Settings (Defaults)
    const [fontSettings, setFontSettings] = createSignal<FontSettings>({
        color: '#ffffff',
        backgroundColor: 'transparent',
        fontSize: 48,
        fontWeight: '400',
        lineHeight: 1.5,
        letterSpacing: 0
    });

    const reset = () => {
        setZoom(100);
        setRotation(0);
        setFlip({ horizontal: false, vertical: false });
        setPosition({ x: 0, y: 0 });
        setTool("pan");
        setMediaType('unknown');
        // Reset Font Settings
        setFontSettings({
            color: '#ffffff',
            backgroundColor: 'transparent',
            fontSize: 48,
            fontWeight: '400',
            lineHeight: 1.5,
            letterSpacing: 0
        });
        // Do NOT reset slideshow state here, as it should persist across item changes
    };

    const value: ItemViewContextState = {
        zoom, setZoom,
        rotation, setRotation,
        flip, setFlip,
        tool, setTool,
        position, setPosition,
        mediaType, setMediaType,
        slideshowPlaying, setSlideshowPlaying,
        slideshowDuration, setSlideshowDuration,
        fontSettings, setFontSettings,
        reset
    };

    return (
        <ItemViewContext.Provider value={value}>
            {props.children}
        </ItemViewContext.Provider>
    );
};

export const useItemViewContext = () => {
    const context = useContext(ItemViewContext);
    if (!context) {
        throw new Error("useItemViewContext must be used within a ItemViewProvider");
    }
    return context;
};
