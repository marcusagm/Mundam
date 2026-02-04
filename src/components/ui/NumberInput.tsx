import { Component, splitProps, mergeProps } from "solid-js";
import { Input, InputProps } from "./Input";
import { Minus, Plus } from "lucide-solid";
import "./number-input.css";
import { cn } from "../../lib/utils";
import { createControllableSignal } from "../../lib/primitives";

export interface NumberInputProps extends Omit<InputProps, "onChange" | "onInput" | "value" | "defaultValue"> {
  value?: number;
  defaultValue?: number;
  min?: number;
  max?: number;
  step?: number;
  onChange?: (value: number | undefined) => void;
  format?: (value: number) => string;
}

export const NumberInput: Component<NumberInputProps> = (props) => {
  const merged = mergeProps({ step: 1, min: -Infinity, max: Infinity }, props);
  // Extract icons to prevent overriding
  const [local, others] = splitProps(merged, [
    "value",
    "defaultValue",
    "min",
    "max",
    "step",
    "onChange",
    "format",
    "class",
    "disabled",
    "leftIcon", 
    "rightIcon"
  ]);

  const { value, setValue } = createControllableSignal<number | undefined>({
    value: () => local.value,
    defaultValue: local.defaultValue,
    onChange: local.onChange,
  });

  const handleInput = (e: InputEvent & { currentTarget: HTMLInputElement }) => {
    const val = e.currentTarget.value;
    if (val === "") {
        setValue(undefined);
        return;
    }
    const num = parseFloat(val);
    if (!isNaN(num)) {
        setValue(num);
    }
  };
  
  const handleKeyDown = (e: KeyboardEvent) => {
    // Allow: backspace, delete, tab, escape, enter, decimal point (if not integer step? assume float for now), minus (if min < 0)
    // Allow: Ctrl+A, Ctrl+C, Ctrl+V, Ctrl+X
    // Allow: home, end, left, right
    
    // Block standard letters
    // Simple regex check on key?
    // Browser "type=number" usually handles this, but user said "accepts characters".
    // Maybe because we passed `type="number"`?
    // If browser implementation is loose, we force it.
    
    const allowedKeys = [
        "Backspace", "Delete", "Tab", "Escape", "Enter", "ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown",
        "Home", "End", "Period", "-", "."
    ];
    
    if (allowedKeys.includes(e.key)) return;
    if (e.ctrlKey || e.metaKey) return;
    
    if (!/^[0-9]$/.test(e.key)) {
        e.preventDefault();
    }
  };

  const increment = () => {
    if (local.disabled) return;
    const current = value() ?? 0;
    const next = Math.min(local.max, current + local.step);
    setValue(next);
  };

  const decrement = () => {
    if (local.disabled) return;
    const current = value() ?? 0;
    const next = Math.max(local.min, current - local.step);
    setValue(next);
  };

  return (
    <Input
      type="number" // Use type="text" and handle validation manually if type="number" allows trash?
      // type="number" is usually best for mobile keyboard.
      // But we will add onKeyDown to be stricter.
      class={cn("ui-number-input", local.class)}
      value={value() ?? ""}
      min={local.min}
      max={local.max}
      step={local.step}
      disabled={local.disabled}
      onInput={handleInput}
      onKeyDown={handleKeyDown}
      leftIcon={
        <button
          type="button"
          class="ui-number-input-btn"
          onClick={decrement}
          disabled={local.disabled || (value() !== undefined && value()! <= local.min)}
          tabIndex={-1}
          aria-label="Decrease value"
        >
          <Minus size={14} />
        </button>
      }
      rightIcon={
        <button
          type="button"
          class="ui-number-input-btn"
          onClick={increment}
          disabled={local.disabled || (value() !== undefined && value()! >= local.max)}
          tabIndex={-1}
          aria-label="Increase value"
        >
          <Plus size={14} />
        </button>
      }
      {...others}
    />
  );
};
