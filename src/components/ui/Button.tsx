import { Component, JSX, splitProps, Show } from "solid-js";
import { cn } from "../../lib/utils";
import "./button.css";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "ghost-destructive" | "destructive" | "outline";
export type ButtonSize = "sm" | "md" | "lg" | "icon" | "icon-sm" | "icon-xs";

export interface ButtonProps extends JSX.ButtonHTMLAttributes<HTMLButtonElement> {
  /** Visual variant */
  variant?: ButtonVariant;
  /** Size variant */
  size?: ButtonSize;
  /** Loading state */
  loading?: boolean;
  /** Icon to display before children */
  leftIcon?: JSX.Element;
  /** Icon to display after children */
  rightIcon?: JSX.Element;
  /** Button content */
  children?: JSX.Element;
}

/**
 * Button component with multiple variants, sizes, and states.
 * 
 * @example
 * <Button>Click me</Button>
 * 
 * @example
 * <Button variant="destructive" loading>
 *   Deleting...
 * </Button>
 * 
 * @example
 * <Button variant="ghost" leftIcon={<Plus size={16} />}>
 *   Add item
 * </Button>
 */
export const Button: Component<ButtonProps> = (props) => {
  const [local, others] = splitProps(props, [
    "variant",
    "size",
    "loading",
    "leftIcon",
    "rightIcon",
    "class",
    "children",
    "disabled",
  ]);

  const variant = () => local.variant || "primary";
  const size = () => local.size || "md";
  const isDisabled = () => local.disabled || local.loading;

  return (
    <button
      class={cn(
        "ui-btn",
        `ui-btn-${variant()}`,
        `ui-btn-${size()}`,
        local.loading && "ui-btn-loading",
        local.class
      )}
      disabled={isDisabled()}
      aria-busy={local.loading || undefined}
      {...others}
    >
      <Show when={local.loading}>
        <span class="ui-btn-spinner" aria-hidden="true" />
      </Show>

      <Show when={local.leftIcon && !local.loading}>
        <span class="ui-btn-icon ui-btn-icon-left" aria-hidden="true">
          {local.leftIcon}
        </span>
      </Show>

      <Show when={local.children}>
        <span class="ui-btn-content">{local.children}</span>
      </Show>

      <Show when={local.rightIcon}>
        <span class="ui-btn-icon ui-btn-icon-right" aria-hidden="true">
          {local.rightIcon}
        </span>
      </Show>
    </button>
  );
};
