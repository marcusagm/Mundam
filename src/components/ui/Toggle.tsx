import { Component, JSX, splitProps } from "solid-js";
import { cn } from "../../lib/utils";
import { createControllableSignal } from "../../lib/primitives";
import "./toggle.css";

type ToggleVariant = "default" | "outline";
type ToggleSize = "sm" | "md" | "lg";

export interface ToggleProps extends Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "onChange"> {
  pressed?: boolean;
  defaultPressed?: boolean;
  onPressedChange?: (pressed: boolean) => void;
  variant?: ToggleVariant;
  size?: ToggleSize;
  children: JSX.Element;
}

/**
 * Toggle button for binary on/off states with visible feedback.
 * Unlike Switch, Toggle is a button that shows pressed state visually.
 * 
 * @example
 * <Toggle aria-label="Toggle bold">
 *   <Bold size={16} />
 * </Toggle>
 */
export const Toggle: Component<ToggleProps> = (props) => {
  const [local, others] = splitProps(props, [
    "class",
    "pressed",
    "defaultPressed",
    "onPressedChange",
    "variant",
    "size",
    "disabled",
    "children",
  ]);

  const { value: isPressed, setValue: setPressed } = createControllableSignal({
    value: () => local.pressed,
    defaultValue: local.defaultPressed ?? false,
    onChange: local.onPressedChange,
  });

  const handleClick = () => {
    if (local.disabled) return;
    setPressed(!isPressed());
  };

  return (
    <button
      type="button"
      class={cn(
        "ui-toggle",
        `ui-toggle-${local.variant || "default"}`,
        `ui-toggle-${local.size || "md"}`,
        isPressed() && "ui-toggle-pressed",
        local.class
      )}
      aria-pressed={isPressed()}
      disabled={local.disabled}
      onClick={handleClick}
      {...others}
    >
      {local.children}
    </button>
  );
};
