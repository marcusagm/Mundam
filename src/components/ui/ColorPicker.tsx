import { Component, createSignal, onMount, For } from "solid-js";
import "./color-picker.css";

// Helper: HSB <-> Hex
// Simple conversion logic for the picker
function hsbToHex(h: number, s: number, b: number): string {
  s /= 100;
  b /= 100;
  const k = (n: number) => (n + h / 60) % 6;
  const f = (n: number) => b * (1 - s * Math.max(0, Math.min(k(n), 4 - k(n), 1)));
  const toHex = (x: number) => Math.round(255 * x).toString(16).padStart(2, '0');
  return `#${toHex(f(5))}${toHex(f(3))}${toHex(f(1))}`;
}

function hexToHsb(hex: string): { h: number, s: number, b: number } {
  hex = hex.replace(/^#/, '');
  if (hex.length === 3) hex = hex.split('').map(c => c + c).join('');
  
  const r = parseInt(hex.slice(0, 2), 16) / 255;
  const g = parseInt(hex.slice(2, 4), 16) / 255;
  const b = parseInt(hex.slice(4, 6), 16) / 255;
  
  const v = Math.max(r, g, b);
  const n = v - Math.min(r, g, b);
  const h = n === 0 ? 0 : n && v === r ? (g - b) / n : v === g ? 2 + (b - r) / n : 4 + (r - g) / n;
  
  return {
    h: 60 * (h < 0 ? h + 6 : h),
    s: v && (n / v) * 100,
    b: v * 100
  };
}

interface ColorPickerProps {
    color: string;
    onChange: (color: string) => void;
}

const PRESET_COLORS = [
    "#ff0000", "#ff4500", "#ffa500", "#ffff00", "#9acd32", "#008000", 
    "#006400", "#00ced1", "#1e90ff", "#0000ff", "#4b0082", "#800080", 
    "#c71585", "#ff1493", "#ffffff", "#808080", "#000000"
];

export const ColorPicker: Component<ColorPickerProps> = (props) => {
    // State
    const [hsb, setHsb] = createSignal({ h: 0, s: 100, b: 100 });
    const [draggingSB, setDraggingSB] = createSignal(false);
    const [draggingHue, setDraggingHue] = createSignal(false);
    
    // Refs
    let sbAreaRef: HTMLDivElement | undefined;
    let hueRef: HTMLDivElement | undefined;

    // Initialize from props
    onMount(() => {
        setHsb(hexToHsb(props.color || "#ff0000"));
    });

    const updateColor = (newHsb: { h: number, s: number, b: number }) => {
        setHsb(newHsb);
        props.onChange(hsbToHex(newHsb.h, newHsb.s, newHsb.b));
    };

    // SB Area Interaction
    const handleSbMove = (e: MouseEvent) => {
        if (!sbAreaRef) return;
        const rect = sbAreaRef.getBoundingClientRect();
        const x = Math.max(0, Math.min(rect.width, e.clientX - rect.left));
        const y = Math.max(0, Math.min(rect.height, e.clientY - rect.top));
        
        const s = (x / rect.width) * 100;
        const b = 100 - (y / rect.height) * 100;
        
        updateColor({ ...hsb(), s, b });
    };

    // Hue Interaction
    const handleHueMove = (e: MouseEvent) => {
        if (!hueRef) return;
        const rect = hueRef.getBoundingClientRect();
        const x = Math.max(0, Math.min(rect.width, e.clientX - rect.left));
        const h = (x / rect.width) * 360;
        
        updateColor({ ...hsb(), h });
    };

    const stopDrag = () => {
        setDraggingSB(false);
        setDraggingHue(false);
        window.removeEventListener('mousemove', onGlobalMove);
        window.removeEventListener('mouseup', stopDrag);
    };

    const onGlobalMove = (e: MouseEvent) => {
        if (draggingSB()) handleSbMove(e);
        if (draggingHue()) handleHueMove(e);
    };

    const startDragSB = (e: MouseEvent) => {
        setDraggingSB(true);
        handleSbMove(e);
        window.addEventListener('mousemove', onGlobalMove);
        window.addEventListener('mouseup', stopDrag);
    };

    const startDragHue = (e: MouseEvent) => {
        setDraggingHue(true);
        handleHueMove(e);
        window.addEventListener('mousemove', onGlobalMove);
        window.addEventListener('mouseup', stopDrag);
    };

    return (
        <div class="color-picker-container" onClick={(e) => e.stopPropagation()}>
            {/* Saturation/Brightness Area (Square) */}
            <div 
                ref={sbAreaRef}
                class="sb-area"
                style={{
                    "background-color": `hsl(${hsb().h}, 100%, 50%)`
                }}
                onMouseDown={startDragSB}
            >
                <div class="sb-layer-white"></div>
                <div class="sb-layer-black"></div>
                {/* Thumb */}
                <div 
                    class="picker-thumb"
                    style={{
                        left: `${hsb().s}%`,
                        top: `${100 - hsb().b}%`
                    }}
                />
            </div>

            {/* Hue Slider */}
            <div 
                ref={hueRef}
                class="hue-slider"
                onMouseDown={startDragHue}
            >
                {/* Thumb */}
                <div 
                    class="picker-thumb-slide"
                    style={{
                        left: `${(hsb().h / 360) * 100}%`
                    }}
                />
            </div>

            {/* Preview & Input */}
            <div style={{ display: "flex", "align-items": "center", gap: "8px" }}>
                <div 
                    class="color-preview-box"
                    style={{ "background-color": props.color }}
                />
                <input 
                    type="text" 
                    value={props.color} 
                    onInput={(e) => {
                        const val = e.currentTarget.value;
                        if (/^#[0-9A-Fa-f]{6}$/.test(val)) {
                            props.onChange(val);
                            setHsb(hexToHsb(val));
                        }
                    }}
                    class="hex-input"
                />
            </div>

            {/* Presets */}
            <div class="color-presets">
                <For each={PRESET_COLORS}>
                    {(c) => (
                        <div 
                            class="color-preset-item" 
                            style={{ "background-color": c }}
                            onClick={() => {
                                props.onChange(c);
                                setHsb(hexToHsb(c));
                            }}
                            title={c}
                        />
                    )}
                </For>
            </div>
        </div>
    );
};
