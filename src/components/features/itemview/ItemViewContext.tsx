import {
    createContext,
    useContext,
    createSignal,
    ParentComponent,
    Accessor,
    Setter
} from 'solid-js';
import { type MediaType } from '../../../core/store/formatStore';

export interface FlipState {
    horizontal: boolean;
    vertical: boolean;
}

export interface Position {
    x: number;
    y: number;
}

export type ViewportTool = 'select' | 'pan' | 'rotate';

interface ItemViewContextState {
    rotation: Accessor<number>;
    setRotation: Setter<number>;
    flip: Accessor<FlipState>;
    setFlip: Setter<FlipState>;
    tool: Accessor<ViewportTool>;
    setTool: Setter<ViewportTool>;
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

    // Model Settings
    modelSettings: Accessor<ModelSettings>;
    setModelSettings: Setter<ModelSettings>;

    resetTrigger: Accessor<number>;
    reset: () => void;
    /** Restores rotation, flip, pan and zoom (fit to screen) keeping the active tool */
    resetView: () => void;
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

export interface ModelSettings {
    autoRotate: boolean;
    showGrid: boolean;
    backgroundColor: string;
}

const ItemViewContext = createContext<ItemViewContextState>();

export const ItemViewProvider: ParentComponent = props => {
    const [rotation, setRotation] = createSignal(0);
    const [flip, setFlip] = createSignal<FlipState>({ horizontal: false, vertical: false });
    const [tool, setTool] = createSignal<ViewportTool>('select');
    const [mediaType, setMediaType] = createSignal<MediaType>('unknown');

    // Timer / Slideshow
    const [slideshowPlaying, setSlideshowPlaying] = createSignal(false);
    const [slideshowDuration, setSlideshowDuration] = createSignal(5); // seconds

    // Font Settings
    const [fontSettings, setFontSettings] = createSignal<FontSettings>({
        color: '#ffffff',
        backgroundColor: 'transparent',
        fontSize: 48,
        fontWeight: '400',
        lineHeight: 1.5,
        letterSpacing: 0
    });

    // Model Settings
    const [modelSettings, setModelSettings] = createSignal<ModelSettings>({
        autoRotate: true,
        showGrid: false,
        backgroundColor: '#111111'
    });

    const [resetTrigger, setResetTrigger] = createSignal(0);

    const reset = () => {
        setRotation(0);
        setFlip({ horizontal: false, vertical: false });
        setTool('select');
        // DO NOT reset mediaType here

        // Reset Settings
        setFontSettings({
            color: '#ffffff',
            backgroundColor: 'transparent',
            fontSize: 48,
            fontWeight: '400',
            lineHeight: 1.5,
            letterSpacing: 0
        });
        setModelSettings({
            autoRotate: true,
            showGrid: false,
            backgroundColor: '#111111'
        });
        // Signal renderers to reset internal state
        setResetTrigger(t => t + 1);
    };

    const resetView = () => {
        setRotation(0);
        setFlip({ horizontal: false, vertical: false });
        // Fit resets zoom and pan to the initial state
        window.dispatchEvent(new CustomEvent('viewport:fit'));
    };

    const value: ItemViewContextState = {
        rotation,
        setRotation,
        flip,
        setFlip,
        tool,
        setTool,
        mediaType,
        setMediaType,
        slideshowPlaying,
        setSlideshowPlaying,
        slideshowDuration,
        setSlideshowDuration,
        fontSettings,
        setFontSettings,
        modelSettings,
        setModelSettings,
        resetTrigger,
        reset,
        resetView
    };

    return <ItemViewContext.Provider value={value}>{props.children}</ItemViewContext.Provider>;
};

export const useItemViewContext = () => {
    const context = useContext(ItemViewContext);
    if (!context) {
        throw new Error('useItemViewContext must be used within a ItemViewProvider');
    }
    return context;
};
