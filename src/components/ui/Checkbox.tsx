import { Component, JSX, splitProps, createMemo, Show } from "solid-js";
import { cn } from "../../lib/utils";
import { createControllableSignal } from "../../lib/primitives";
import { createId } from "../../lib/primitives/createId";
import { Check, Minus } from "lucide-solid";
import "./checkbox.css";

export interface CheckboxProps extends Omit<JSX.InputHTMLAttributes<HTMLInputElement>, "onChange" | "type"> {
  checked?: boolean;
  defaultChecked?: boolean;
  indeterminate?: boolean;
  onCheckedChange?: (checked: boolean) => void;
  label?: string;
  description?: string;
  size?: "sm" | "md" | "lg";
}

/**
 * Checkbox component for selecting multiple options.
 * Supports indeterminate state for partial selections.
 * 
 * @example
 * <Checkbox label="Accept terms" />
 * <Checkbox checked indeterminate label="Select all" />
 */
export const Checkbox: Component<CheckboxProps> = (props) => {
  const [local, others] = splitProps(props, [
    "class",
    "checked",
    "defaultChecked",
    "indeterminate",
    "onCheckedChange",
    "label",
    "description",
    "size",
    "id",
    "disabled",
  ]);

  const id = createMemo(() => local.id || createId("checkbox"));
  const descriptionId = createMemo(() => local.description ? `${id()}-desc` : undefined);

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

  const showCheck = () => isChecked() && !local.indeterminate;
  const showIndeterminate = () => local.indeterminate;

  return (
    <label
      class={cn(
        "ui-checkbox-wrapper",
        local.disabled && "ui-checkbox-disabled",
        local.class
      )}
      for={id()}
    >
      <button
        type="button"
        role="checkbox"
        id={id()}
        class={cn(
          "ui-checkbox",
          `ui-checkbox-${local.size || "md"}`,
          (isChecked() || local.indeterminate) && "ui-checkbox-checked"
        )}
        aria-checked={local.indeterminate ? "mixed" : isChecked()}
        aria-disabled={local.disabled}
        aria-describedby={descriptionId()}
        disabled={local.disabled}
        onClick={handleClick}
        onKeyDown={handleKeyDown}
      >
        <Show when={showCheck()}>
          <Check size={12} class="ui-checkbox-icon" />
        </Show>
        <Show when={showIndeterminate()}>
          <Minus size={12} class="ui-checkbox-icon" />
        </Show>
      </button>

      <Show when={local.label || local.description}>
        <div class="ui-checkbox-content">
          <Show when={local.label}>
            <span class="ui-checkbox-label">{local.label}</span>
          </Show>
          <Show when={local.description}>
            <span id={descriptionId()} class="ui-checkbox-description">
              {local.description}
            </span>
          </Show>
        </div>
      </Show>

      {/* Hidden input for form submission */}
      <input
        type="checkbox"
        checked={isChecked()}
        disabled={local.disabled}
        class="ui-checkbox-input"
        tabindex={-1}
        aria-hidden="true"
        {...others}
      />
    </label>
  );
};
