import { Component, splitProps, mergeProps, createSignal, createEffect } from "solid-js";
import { InputProps } from "./Input";
import { ColorPicker } from "./ColorPicker";
import { Popover } from "./Popover";
import { cn } from "../../lib/utils";
import "./color-input.css";
import { createControllableSignal } from "../../lib/primitives";

export interface ColorInputProps extends Omit<InputProps, "onChange" | "onInput" | "value" | "defaultValue"> {
  value?: string;
  defaultValue?: string;
  onChange?: (color: string) => void;
  size?: "sm" | "md" | "lg";
  /** Whether to show the alpha channel (not fully supported by ColorPicker yet, assumes Hex) */
  // Presets can be passed to ColorPicker if we exposed them, currently not exposed in ColorInputProps but could be.
}

const isValidHex = (hex: string): boolean => {
  return /^#([0-9A-Fa-f]{3}|[0-9A-Fa-f]{6})$/.test(hex);
};

export const ColorInput: Component<ColorInputProps> = (props) => {
  const merged = mergeProps({ defaultValue: "#000000" }, props);
  const [local, others] = splitProps(merged, [
    "value",
    "defaultValue",
    "onChange",
    "class",
    "wrapperClass",
    "label",
    "error",
    "errorMessage",
    "disabled",
    "size"
  ]);

  const { value, setValue } = createControllableSignal({
    value: () => local.value,
    defaultValue: local.defaultValue,
    onChange: local.onChange,
  });

  // Internal signal for input field to allow typing partial hex before validation
  const [inputValue, setInputValue] = createSignal(value() || "");
  
  createEffect(() => {
    // If value prop changes externally, update input text
    const v = value();
    if (v) {
        setInputValue(v);
    }
  });

  const handleInput = (e: InputEvent & { currentTarget: HTMLInputElement }) => {
    const val = e.currentTarget.value;
    setInputValue(val);
    
    // Live update if valid
    if (isValidHex(val)) {
        setValue(val);
    }
  };

  const handleBlur = () => {
    let val = inputValue();
    if (!val.startsWith("#") && /^[0-9A-Fa-f]{3,6}$/.test(val)) {
        val = "#" + val;
    }
    
    if (isValidHex(val)) {
        // Expand 3 char hex
        if (val.length === 4) {
            val = "#" + val[1] + val[1] + val[2] + val[2] + val[3] + val[3];
        }
        setValue(val);
        setInputValue(val);
    } else {
        setInputValue(value() || "#000000");
    }
  };

  const handlePickerChange = (color: string) => {
    setValue(color);
    setInputValue(color);
  };
  
  const triggerButton = (
    <button
      type="button"
      class="ui-color-input-swatch-btn"
      disabled={local.disabled}
      aria-label="Open color picker"
    >
      <div 
        class="ui-color-input-swatch" 
        style={{ 
            "background-color": value(),
            "background-image": value() === "transparent" ? "linear-gradient(45deg, #ccc 25%, transparent 25%), linear-gradient(-45deg, #ccc 25%, transparent 25%), linear-gradient(45deg, transparent 75%, #ccc 75%), linear-gradient(-45deg, transparent 75%, #ccc 75%)" : "none",
            "background-size": "8px 8px",
            "background-position": "0 0, 0 4px, 4px -4px, -4px 0px"
        }} 
      />
    </button>
  );

  return (
    <div class={cn("ui-color-input-wrapper", local.wrapperClass)}>
      {local.label && (
        <label class="ui-color-input-label">{local.label}</label>
      )}
      <div
        class={cn(
          "ui-color-input-container",
          `ui-color-input-${local.size || "md"}`,
          local.error && "ui-color-input-error",
          local.disabled && "ui-color-input-disabled",
          local.class 
        )}
      >
        <div class="ui-color-input-icon-left">
          {local.disabled ? (
              triggerButton
          ) : (
            <Popover
                  trigger={triggerButton}
                  class="ui-color-input-popover"
              >
              <ColorPicker color={value() || "#000000"} onChange={handlePickerChange} />
            </Popover>
          )}
        </div>

        <input
          class="ui-color-input-field"
          value={inputValue()}
          onInput={handleInput}
          onBlur={handleBlur}
          disabled={local.disabled}
          maxLength={9} // #RRGGBBAA
          aria-invalid={local.error || undefined}
          {...others}
        />


      </div>

      {local.error && local.errorMessage && (
        <span class="ui-color-input-error-message" role="alert">
          {local.errorMessage}
        </span>
      )}
    </div>
  );
};
