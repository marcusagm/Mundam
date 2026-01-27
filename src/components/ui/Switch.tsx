import { Component, JSX, splitProps, createMemo } from "solid-js";
import { cn } from "../../lib/utils";
import { createControllableSignal } from "../../lib/primitives";
import { createId } from "../../lib/primitives/createId";
import "./switch.css";

export interface SwitchProps extends Omit<JSX.InputHTMLAttributes<HTMLInputElement>, "onChange" | "type"> {
  checked?: boolean;
  defaultChecked?: boolean;
  onCheckedChange?: (checked: boolean) => void;
  label?: string;
  size?: "sm" | "md" | "lg";
}

/**
 * Switch component for toggling between two states.
 * Follows WAI-ARIA switch pattern for accessibility.
 * 
 * @example
 * // Uncontrolled
 * <Switch defaultChecked label="Enable notifications" />
 * 
 * // Controlled
 * const [enabled, setEnabled] = createSignal(false);
 * <Switch checked={enabled()} onCheckedChange={setEnabled} />
 */
export const Switch: Component<SwitchProps> = (props) => {
  const [local, others] = splitProps(props, [
    "class",
    "checked",
    "defaultChecked",
    "onCheckedChange",
    "label",
    "size",
    "id",
    "disabled",
  ]);

  const id = createMemo(() => local.id || createId("switch"));
  
  const { value: isChecked, setValue: setChecked } = createControllableSignal({
    value: () => local.checked,
    defaultValue: local.defaultChecked ?? false,
    onChange: local.onCheckedChange,
  });

  const handleClick = () => {
    if (local.disabled) return;
    setChecked(!isChecked());
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (local.disabled) return;
    if (e.key === " " || e.key === "Enter") {
      e.preventDefault();
      setChecked(!isChecked());
    }
  };

  return (
    <label
      class={cn(
        "ui-switch-wrapper",
        local.disabled && "ui-switch-disabled",
        local.class
      )}
      for={id()}
    >
      <button
        type="button"
        role="switch"
        id={id()}
        class={cn(
          "ui-switch",
          `ui-switch-${local.size || "md"}`,
          isChecked() && "ui-switch-checked"
        )}
        aria-checked={isChecked()}
        aria-disabled={local.disabled}
        disabled={local.disabled}
        onClick={handleClick}
        onKeyDown={handleKeyDown}
      >
        <span class="ui-switch-thumb" />
      </button>
      {local.label && <span class="ui-switch-label">{local.label}</span>}
      
      {/* Hidden input for form submission */}
      <input
        type="checkbox"
        checked={isChecked()}
        disabled={local.disabled}
        class="ui-switch-input"
        tabindex={-1}
        aria-hidden="true"
        {...others}
      />
    </label>
  );
};
