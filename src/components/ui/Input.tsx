import { Component, JSX, splitProps, Show } from "solid-js";
import { cn } from "../../lib/utils";
import "./input.css";

export type InputSize = "sm" | "md" | "lg";

export interface InputProps extends JSX.InputHTMLAttributes<HTMLInputElement> {
  /** Icon to display on the left side */
  leftIcon?: JSX.Element;
  /** Icon to display on the right side */
  rightIcon?: JSX.Element;
  /** Size variant */
  size?: InputSize;
  /** Error state */
  error?: boolean;
  /** Error message to display */
  errorMessage?: string;
  /** Wrapper class */
  wrapperClass?: string;
}

/**
 * Input component with support for icons, sizes, and error states.
 * 
 * @example
 * // Basic usage
 * <Input placeholder="Enter your name" />
 * 
 * @example
 * // With icon
 * <Input leftIcon={<Search size={16} />} placeholder="Search..." />
 * 
 * @example
 * // With error
 * <Input error errorMessage="This field is required" />
 */
export const Input: Component<InputProps> = (props) => {
  const [local, others] = splitProps(props, [
    "leftIcon",
    "rightIcon",
    "size",
    "error",
    "errorMessage",
    "wrapperClass",
    "class",
  ]);

  const size = () => local.size || "md";

  return (
    <div class={cn("ui-input-wrapper", local.wrapperClass)}>
      <div
        class={cn(
          "ui-input-container",
          `ui-input-${size()}`,
          local.error && "ui-input-error",
          !!local.leftIcon && "ui-input-has-left",
          !!local.rightIcon && "ui-input-has-right"
        )}
      >
        <Show when={local.leftIcon}>
          <span class="ui-input-icon ui-input-icon-left" aria-hidden="true">
            {local.leftIcon}
          </span>
        </Show>

        <input
          class={cn("ui-input", local.class)}
          aria-invalid={local.error || undefined}
          aria-describedby={local.error && local.errorMessage ? `${others.id}-error` : undefined}
          {...others}
        />

        <Show when={local.rightIcon}>
          <span class="ui-input-icon ui-input-icon-right" aria-hidden="true">
            {local.rightIcon}
          </span>
        </Show>
      </div>

      <Show when={local.error && local.errorMessage}>
        <span
          id={`${others.id}-error`}
          class="ui-input-error-message"
          role="alert"
        >
          {local.errorMessage}
        </span>
      </Show>
    </div>
  );
};
