import { Component, JSX, splitProps, Show } from "solid-js";
import { Dynamic } from "solid-js/web";
import { cn } from "../../lib/utils";
import { AlertCircle, CheckCircle2, AlertTriangle, Info, X } from "lucide-solid";
import "./alert.css";

type AlertVariant = "default" | "info" | "success" | "warning" | "destructive";

export interface AlertProps extends JSX.HTMLAttributes<HTMLDivElement> {
  variant?: AlertVariant;
  icon?: Component<{ size?: number | string }>;
  title?: string;
  dismissible?: boolean;
  onDismiss?: () => void;
  children?: JSX.Element;
}

const variantIcons: Record<AlertVariant, Component<{ size?: number | string }>> = {
  default: Info,
  info: Info,
  success: CheckCircle2,
  warning: AlertTriangle,
  destructive: AlertCircle,
};

/**
 * Alert component for displaying important messages.
 * Supports multiple variants and optional dismissal.
 * 
 * @example
 * <Alert variant="success" title="Success!">
 *   Your changes have been saved.
 * </Alert>
 * 
 * <Alert variant="destructive" dismissible onDismiss={() => {}}>
 *   Something went wrong.
 * </Alert>
 */
export const Alert: Component<AlertProps> = (props) => {
  const [local, others] = splitProps(props, [
    "class", 
    "variant", 
    "icon", 
    "title", 
    "dismissible", 
    "onDismiss", 
    "children"
  ]);

  const variant = () => local.variant || "default";
  const IconComponent = () => local.icon || variantIcons[variant()];

  return (
    <div
      class={cn("ui-alert", `ui-alert-${variant()}`, local.class)}
      role="alert"
      {...others}
    >
      <span class="ui-alert-icon">
        <Dynamic component={IconComponent()} />
      </span>
      
      <div class="ui-alert-content">
        <Show when={local.title}>
          <h5 class="ui-alert-title">{local.title}</h5>
        </Show>
        <Show when={local.children}>
          <div class="ui-alert-description">{local.children}</div>
        </Show>
      </div>

      <Show when={local.dismissible}>
        <button
          type="button"
          class="ui-alert-dismiss"
          onClick={local.onDismiss}
          aria-label="Dismiss"
        >
          <X size={14} />
        </button>
      </Show>
    </div>
  );
};

export const AlertTitle: Component<{ children: JSX.Element }> = (props) => (
  <h5 class="ui-alert-title">{props.children}</h5>
);

export const AlertDescription: Component<{ children: JSX.Element }> = (props) => (
  <div class="ui-alert-description">{props.children}</div>
);
