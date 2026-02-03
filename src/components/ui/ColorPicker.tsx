import { 
  Component, 
  createSignal, 
  onMount, 
  onCleanup,
  For, 
  splitProps,
  createEffect,
} from "solid-js";
import { cn } from "../../lib/utils";
import "./color-picker.css";

// Color conversion utilities
function hsbToHex(h: number, s: number, b: number): string {
  s /= 100;
  b /= 100;
  const k = (n: number) => (n + h / 60) % 6;
  const f = (n: number) => b * (1 - s * Math.max(0, Math.min(k(n), 4 - k(n), 1)));
  const toHex = (x: number) => Math.round(255 * x).toString(16).padStart(2, "0");
  return `#${toHex(f(5))}${toHex(f(3))}${toHex(f(1))}`;
}

function hexToHsb(hex: string): { h: number; s: number; b: number } {
  hex = hex.replace(/^#/, "");
  if (hex.length === 3) {
    hex = hex.split("").map((c) => c + c).join("");
  }

  const r = parseInt(hex.slice(0, 2), 16) / 255;
  const g = parseInt(hex.slice(2, 4), 16) / 255;
  const bVal = parseInt(hex.slice(4, 6), 16) / 255;

  const v = Math.max(r, g, bVal);
  const n = v - Math.min(r, g, bVal);
  const hue =
    n === 0
      ? 0
      : v === r
      ? (g - bVal) / n
      : v === g
      ? 2 + (bVal - r) / n
      : 4 + (r - g) / n;

  return {
    h: 60 * (hue < 0 ? hue + 6 : hue),
    s: v ? (n / v) * 100 : 0,
    b: v * 100,
  };
}

function isValidHex(hex: string): boolean {
  return /^#([0-9A-Fa-f]{3}|[0-9A-Fa-f]{6})$/.test(hex);
}

export interface ColorPickerProps {
  /** Current color value (hex format) */
  color: string;
  /** Callback when color changes */
  onChange: (color: string) => void;
  /** Preset colors to display */
  presets?: string[];
  /** Whether to show the hex input */
  showInput?: boolean;
  /** Additional CSS class */
  class?: string;
  /** Allow selecting no color (transparent) */
  allowNoColor?: boolean;
}

const DEFAULT_PRESETS = [
  "#ef4444", "#f97316", "#f59e0b", "#eab308", "#84cc16", "#22c55e",
  "#10b981", "#14b8a6", "#06b6d4", "#0ea5e9", "#3b82f6", "#6366f1",
  "#8b5cf6", "#a855f7", "#d946ef", "#ec4899", "#f43f5e",
  "#ffffff", "#94a3b8", "#64748b", "#475569", "#1e293b", "#000000",
];

export const ColorPicker: Component<ColorPickerProps> = (props) => {
  const [local] = splitProps(props, [
    "color",
    "onChange",
    "presets",
    "showInput",
    "class",
    "allowNoColor",
  ]);

  const [hsb, setHsb] = createSignal({ h: 0, s: 100, b: 100 });
  const [isDraggingSB, setIsDraggingSB] = createSignal(false);
  const [isDraggingHue, setIsDraggingHue] = createSignal(false);
  const [hexInput, setHexInput] = createSignal("");

  let sbAreaRef: HTMLDivElement | undefined;
  let hueRef: HTMLDivElement | undefined;

  const presets = () => local.presets ?? DEFAULT_PRESETS;
  const showInput = () => local.showInput ?? true;
  const allowNoColor = () => local.allowNoColor ?? false;

  // Initialize from props
  onMount(() => {
    if (local.color === "transparent") {
        setHsb({ h: 0, s: 0, b: 100 }); // Default white-ish if returning from transparent
        setHexInput("transparent");
    } else {
        const initialHsb = hexToHsb(local.color || "#ff0000");
        setHsb(initialHsb);
        setHexInput(local.color);
    }
  });

  createEffect(() => {
      // Sync external color changes back to input if needed (optional)
      if (local.color === "transparent") {
          setHexInput("transparent");
      } else if (isValidHex(local.color) && local.color !== hexInput()) {
        if (!isDraggingSB() && !isDraggingHue()) { // Avoid fighting user input
             // setHexInput(local.color); // This might cause loop or cursor jump. Let's rely on updateColor.
        }
      }
  });

  const updateColor = (newHsb: { h: number; s: number; b: number }) => {
    setHsb(newHsb);
    const hex = hsbToHex(newHsb.h, newHsb.s, newHsb.b);
    setHexInput(hex);
    local.onChange(hex);
  };

  const setTransparent = () => {
      local.onChange("transparent");
      setHexInput("transparent");
      // keep HSB as is or reset? Keep as is so if they pick color again it starts somewhat reasonable.
  };

  // ... (Interaction Handlers remain same, but check hexToHsb safety if needed)

  // SB Area interaction
  const handleSbMove = (e: MouseEvent | PointerEvent) => {
    if (!sbAreaRef) return;
    const rect = sbAreaRef.getBoundingClientRect();
    const x = Math.max(0, Math.min(rect.width, e.clientX - rect.left));
    const y = Math.max(0, Math.min(rect.height, e.clientY - rect.top));

    const s = (x / rect.width) * 100;
    const b = 100 - (y / rect.height) * 100;

    updateColor({ ...hsb(), s, b });
  };

  // Hue slider interaction
  const handleHueMove = (e: MouseEvent | PointerEvent) => {
    if (!hueRef) return;
    const rect = hueRef.getBoundingClientRect();
    const x = Math.max(0, Math.min(rect.width, e.clientX - rect.left));
    const h = (x / rect.width) * 360;

    updateColor({ ...hsb(), h });
  };

  const handleGlobalMove = (e: MouseEvent) => {
    if (isDraggingSB()) handleSbMove(e);
    if (isDraggingHue()) handleHueMove(e);
  };

  const handleGlobalUp = () => {
    setIsDraggingSB(false);
    setIsDraggingHue(false);
    document.removeEventListener("mousemove", handleGlobalMove);
    document.removeEventListener("mouseup", handleGlobalUp);
  };

  const startDragSB = (e: MouseEvent) => {
    setIsDraggingSB(true);
    handleSbMove(e);
    document.addEventListener("mousemove", handleGlobalMove);
    document.addEventListener("mouseup", handleGlobalUp);
  };

  const startDragHue = (e: MouseEvent) => {
    setIsDraggingHue(true);
    handleHueMove(e);
    document.addEventListener("mousemove", handleGlobalMove);
    document.addEventListener("mouseup", handleGlobalUp);
  };

  const handleHexInputChange = (value: string) => {
    setHexInput(value);
    if (allowNoColor() && value.toLowerCase() === "transparent") {
        local.onChange("transparent");
        return;
    }
    if (isValidHex(value)) {
      const newHsb = hexToHsb(value);
      setHsb(newHsb);
      local.onChange(value.toLowerCase());
    }
  };

  // Keyboard Support
  const handleSbKey = (e: KeyboardEvent) => {
    const step = e.shiftKey ? 10 : 1;
    const current = hsb();
    switch (e.key) {
      case "ArrowRight": e.preventDefault(); updateColor({ ...current, s: Math.min(100, current.s + step) }); break;
      case "ArrowLeft": e.preventDefault(); updateColor({ ...current, s: Math.max(0, current.s - step) }); break;
      case "ArrowUp": e.preventDefault(); updateColor({ ...current, b: Math.min(100, current.b + step) }); break;
      case "ArrowDown": e.preventDefault(); updateColor({ ...current, b: Math.max(0, current.b - step) }); break;
    }
  };

  const handleHueKey = (e: KeyboardEvent) => {
    const step = e.shiftKey ? 10 : 1;
    const current = hsb();
    switch (e.key) {
      case "ArrowRight": e.preventDefault(); updateColor({ ...current, h: (current.h + step) % 360 }); break;
      case "ArrowLeft": e.preventDefault(); updateColor({ ...current, h: (current.h - step + 360) % 360 }); break;
    }
  };


  onCleanup(() => {
    document.removeEventListener("mousemove", handleGlobalMove);
    document.removeEventListener("mouseup", handleGlobalUp);
  });

  return (
    <div
      class={cn("ui-color-picker", local.class)}
      onClick={(e) => e.stopPropagation()}
    >
      {/* Saturation/Brightness Area */}
      <div
        ref={sbAreaRef}
        class="ui-color-picker-sb"
        style={{ "background-color": `hsl(${hsb().h}, 100%, 50%)` }}
        onMouseDown={startDragSB}
        onKeyDown={handleSbKey}
        tabindex={0}
        role="slider"
        aria-label="Saturation and brightness"
        aria-valuetext={`Saturation ${Math.round(hsb().s)}%, Brightness ${Math.round(hsb().b)}%`}
      >
        <div class="ui-color-picker-sb-white" />
        <div class="ui-color-picker-sb-black" />
        <div
          class="ui-color-picker-thumb"
          style={{
            left: `${hsb().s}%`,
            top: `${100 - hsb().b}%`,
            display: local.color === "transparent" ? "none" : "block"
          }}
        />
      </div>

      {/* Hue Slider */}
      <div
        ref={hueRef}
        class="ui-color-picker-hue"
        onMouseDown={startDragHue}
        onKeyDown={handleHueKey}
        tabindex={0}
        role="slider"
        aria-label="Hue"
        aria-valuemin={0}
        aria-valuemax={360}
        aria-valuenow={Math.round(hsb().h)}
      >
        <div
          class="ui-color-picker-hue-thumb"
          style={{ 
              left: `${(hsb().h / 360) * 100}%`,
              display: local.color === "transparent" ? "none" : "block"
           }}
        />
      </div>

      {/* Preview & Input */}
      {showInput() && (
        <div class="ui-color-picker-controls">
          <div
            class="ui-color-picker-preview"
            style={{ 
                "background-color": local.color === "transparent" ? "transparent" : local.color,
                "background-image": local.color === "transparent" ? "linear-gradient(45deg, #ccc 25%, transparent 25%), linear-gradient(-45deg, #ccc 25%, transparent 25%), linear-gradient(45deg, transparent 75%, #ccc 75%), linear-gradient(-45deg, transparent 75%, #ccc 75%)" : "none",
                "background-size": "8px 8px", 
                "background-position": "0 0, 0 4px, 4px -4px, -4px 0px"
            }}
            aria-label={`Selected color: ${local.color}`}
          />
          <input
            type="text"
            class="ui-color-picker-input"
            value={hexInput()}
            onInput={(e) => handleHexInputChange(e.currentTarget.value)}
            maxLength={11} 
            spellcheck={false}
            aria-label="Hex color value"
          />
        </div>
      )}

      {/* Presets */}
      <div class="ui-color-picker-presets" role="listbox" aria-label="Color presets">
        {allowNoColor() && (
             <button
               type="button"
               class={cn(
                 "ui-color-picker-preset",
                 "ui-color-picker-preset-transparent",
                 local.color === "transparent" && "ui-color-picker-preset-selected"
               )}
               onClick={setTransparent}
               title="No Color"
               role="option"
               aria-selected={local.color === "transparent"}
               aria-label="Transparent"
             />
        )}
        <For each={presets()}>
          {(color) => (
            <button
              type="button"
              class={cn(
                "ui-color-picker-preset",
                local.color.toLowerCase() === color.toLowerCase() && "ui-color-picker-preset-selected"
              )}
              style={{ "background-color": color }}
              onClick={() => {
                local.onChange(color);
                setHsb(hexToHsb(color));
                setHexInput(color);
              }}
              title={color}
              role="option"
              aria-selected={local.color.toLowerCase() === color.toLowerCase()}
              aria-label={color}
            />
          )}
        </For>
      </div>
    </div>
  );
};
