import {
  Component,
  createSignal,
  For,
  Show,
  createEffect,
  onCleanup,
} from "solid-js";
import { Portal } from "solid-js/web";
import { X, CheckCircle2, AlertCircle, AlertTriangle, Info } from "lucide-solid";
import { cn } from "../../lib/utils";
import "./sonner.css";

// Toast types
export type ToastType = "default" | "success" | "error" | "warning" | "info";

export interface Toast {
  id: string;
  type: ToastType;
  title: string;
  description?: string;
  duration?: number;
  dismissible?: boolean;
  action?: {
    label: string;
    onClick: () => void;
  };
}

interface ToastState {
  toasts: Toast[];
}

// Global toast state (singleton pattern)
const [toastState, setToastState] = createSignal<ToastState>({ toasts: [] });

let toastCounter = 0;

/**
 * Creates a toast notification.
 * Call these functions from anywhere in your app.
 */
export const toast = {
  default: (title: string, options?: Partial<Omit<Toast, "id" | "type" | "title">>) =>
    addToast({ type: "default", title, ...options }),
  
  success: (title: string, options?: Partial<Omit<Toast, "id" | "type" | "title">>) =>
    addToast({ type: "success", title, ...options }),
  
  error: (title: string, options?: Partial<Omit<Toast, "id" | "type" | "title">>) =>
    addToast({ type: "error", title, ...options }),
  
  warning: (title: string, options?: Partial<Omit<Toast, "id" | "type" | "title">>) =>
    addToast({ type: "warning", title, ...options }),
  
  info: (title: string, options?: Partial<Omit<Toast, "id" | "type" | "title">>) =>
    addToast({ type: "info", title, ...options }),
  
  dismiss: (id: string) => removeToast(id),
  
  dismissAll: () => setToastState({ toasts: [] }),
};

function addToast(partial: Omit<Toast, "id">): string {
  const id = `toast-${++toastCounter}`;
  const newToast: Toast = {
    id,
    duration: 15000,
    dismissible: true,
    ...partial,
  };

  setToastState((prev) => ({
    toasts: [...prev.toasts, newToast],
  }));

  return id;
}

function removeToast(id: string) {
  setToastState((prev) => ({
    toasts: prev.toasts.filter((t) => t.id !== id),
  }));
}

// Icons map
const toastIcons: Record<ToastType, Component<{ size?: number }>> = {
  default: Info,
  success: CheckCircle2,
  error: AlertCircle,
  warning: AlertTriangle,
  info: Info,
};

// Individual Toast Component
const ToastItem: Component<{ 
  toast: Toast; 
  index: number; 
  total: number;
  isExpanded: boolean;
}> = (props) => {
  const [isExiting, setIsExiting] = createSignal(false);

  const dismiss = () => {
    setIsExiting(true);
    setTimeout(() => removeToast(props.toast.id), 200);
  };

  // Auto dismiss
  createEffect(() => {
    if (props.toast.duration && props.toast.duration > 0) {
      const timer = setTimeout(dismiss, props.toast.duration);
      onCleanup(() => clearTimeout(timer));
    }
  });

  const IconComponent = toastIcons[props.toast.type];

  // We use reverse index (0 = front, 1 = second, etc.)
  const reverseIndex = () => props.total - 1 - props.index;

  return (
    <div
      class={cn(
        "ui-toast",
        `ui-toast-${props.toast.type}`,
        isExiting() && "ui-toast-exiting"
      )}
      role="alert"
      aria-live="polite"
      data-index={reverseIndex()}
      data-expanded={props.isExpanded}
      style={{
          "--index": reverseIndex(),
          "z-index": props.index,
          "opacity": reverseIndex() >= 3 && !props.isExpanded ? 0 : 1,
          "pointer-events": reverseIndex() > 0 && !props.isExpanded ? "none" : "auto"
      }}
    >
      <div class="ui-toast-icon">
        <IconComponent />
      </div>

      <div class="ui-toast-content">
        <div class="ui-toast-title">{props.toast.title}</div>
        <Show when={props.toast.description}>
          <div class="ui-toast-description">{props.toast.description}</div>
        </Show>
      </div>

      <div class="ui-toast-actions">
        <Show when={props.toast.action}>
          <button
            class="ui-toast-action-btn"
            onClick={() => {
              props.toast.action!.onClick();
              dismiss();
            }}
          >
            {props.toast.action!.label}
          </button>
        </Show>

        <Show when={props.toast.dismissible}>
          <button
            class="ui-toast-close"
            onClick={dismiss}
            aria-label="Dismiss"
          >
            <X size={14} />
          </button>
        </Show>
      </div>
    </div>
  );
};

// Toaster Container Component
export interface ToasterProps {
  position?: "top-left" | "top-center" | "top-right" | "bottom-left" | "bottom-center" | "bottom-right";
  expand?: boolean;
  richColors?: boolean;
}

/**
 * Toaster container component. Place once at app root.
 * 
 * @example
 * // In App.tsx
 * <Toaster position="bottom-right" />
 * 
 * // Anywhere in your app
 * toast.success("Saved successfully!");
 * toast.error("Something went wrong");
 */
export const Toaster: Component<ToasterProps> = (props) => {
  const position = () => props.position || "bottom-right";
  const [isExpanded, setIsExpanded] = createSignal(false);

  return (
    <Portal>
      <div
        class={cn(
          "ui-toaster",
          `ui-toaster-${position()}`,
          props.expand && "ui-toaster-expand",
          props.richColors && "ui-toaster-rich"
        )}
        data-expanded={isExpanded()}
        onMouseEnter={() => setIsExpanded(true)}
        onMouseLeave={() => setIsExpanded(false)}
      >
        <For each={toastState().toasts}>
          {(t, index) => (
            <ToastItem 
              toast={t} 
              index={index()} 
              total={toastState().toasts.length}
              isExpanded={isExpanded()}
            />
          )}
        </For>
      </div>
    </Portal>
  );
};

// Sonner is an alias for Toaster (matching the library name)
export const Sonner = Toaster;
