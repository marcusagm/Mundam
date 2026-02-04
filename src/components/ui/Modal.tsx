import { 
  Component, 
  JSX, 
  Show, 
  createEffect, 
  onCleanup, 
  splitProps,
} from "solid-js";
import { Portal } from "solid-js/web";
import { X } from "lucide-solid";
import { cn } from "../../lib/utils";
import { createFocusTrap } from "../../lib/primitives";
import { createId } from "../../lib/primitives/createId";
import { useShortcuts, createConditionalScope } from "../../core/input";
import { Button } from "./Button";
import "./modal.css";

export type ModalSize = "sm" | "md" | "lg" | "xl" | "full";

export interface ModalProps {
  /** Whether the modal is open */
  isOpen: boolean;
  /** Callback when modal should close */
  onClose: () => void;
  /** Modal title */
  title?: string;
  /** Size variant */
  size?: ModalSize;
  /** Whether clicking the overlay closes the modal (default: true) */
  closeOnOverlayClick?: boolean;
  /** Whether to show the close button (default: true) */
  showCloseButton?: boolean;
  /** Modal content */
  children: JSX.Element;
  /** Footer content */
  footer?: JSX.Element;
  /** Additional CSS class for the container */
  class?: string;
}

/**
 * Modal component for dialogs and overlays.
 * Implements focus trapping and keyboard accessibility.
 * 
 * @example
 * <Modal isOpen={isOpen()} onClose={() => setIsOpen(false)} title="Settings">
 *   <p>Modal content goes here</p>
 * </Modal>
 * 
 * @example
 * <Modal 
 *   isOpen={isOpen()} 
 *   onClose={handleClose} 
 *   title="Confirm Action"
 *   footer={
 *     <>
 *       <Button variant="ghost" onClick={handleClose}>Cancel</Button>
 *       <Button onClick={handleConfirm}>Confirm</Button>
 *     </>
 *   }
 * >
 *   Are you sure you want to proceed?
 * </Modal>
 */
export const Modal: Component<ModalProps> = (props) => {
  const [local] = splitProps(props, [
    "isOpen",
    "onClose",
    "title",
    "size",
    "closeOnOverlayClick",
    "showCloseButton",
    "children",
    "footer",
    "class",
  ]);

  let containerRef: HTMLDivElement | undefined;

  const size = () => local.size || "md";
  const closeOnOverlayClick = () => local.closeOnOverlayClick ?? true;
  const showCloseButton = () => local.showCloseButton ?? true;

  const titleId = createId("modal-title");

  // Focus trap
  createFocusTrap(() => containerRef, () => local.isOpen);

  // Input System Integration
  createConditionalScope("modal", () => local.isOpen, undefined, true);
  
  useShortcuts([
    {
      keys: "Escape",
      name: "Close Modal",
      scope: "modal", // High priority (100)
      enabled: () => local.isOpen,
      action: () => {
        // Only close if it's the top-most modal/scope interaction
        // The scope priority handles this mostly, but good to be safe
        local.onClose();
      }
    }
  ]);

  // Handle body scroll lock
  createEffect(() => {
    if (!local.isOpen) return;

    document.body.style.overflow = "hidden";

    onCleanup(() => {
      document.body.style.overflow = "";
    });
  });

  // Focus the container when opening
  createEffect(() => {
    if (local.isOpen && containerRef) {
      containerRef.focus();
    }
  });

  const handleOverlayClick = (e: MouseEvent) => {
    if (e.target === e.currentTarget && closeOnOverlayClick()) {
      local.onClose();
    }
  };

  return (
    <Show when={local.isOpen}>
      <Portal>
        <div
          class="ui-modal-overlay"
          onClick={handleOverlayClick}
          aria-hidden="true"
        />
        <div
          ref={containerRef}
          class={cn(
            "ui-modal-container",
            `ui-modal-${size()}`,
            local.class
          )}
          role="dialog"
          aria-modal="true"
          aria-labelledby={local.title ? titleId : undefined}
          tabindex={-1}
        >
          <Show when={local.title || showCloseButton()}>
            <header class="ui-modal-header">
              <Show when={local.title}>
                <h2 id={titleId} class="ui-modal-title">
                  {local.title}
                </h2>
              </Show>
              <Show when={showCloseButton()}>
                <button
                  type="button"
                  class="ui-modal-close"
                  onClick={local.onClose}
                  aria-label="Close modal"
                >
                  <X size={18} />
                </button>
              </Show>
            </header>
          </Show>

          <div class="ui-modal-body">
            {local.children}
          </div>

          <Show when={local.footer}>
            <footer class="ui-modal-footer">
              {local.footer}
            </footer>
          </Show>
        </div>
      </Portal>
    </Show>
  );
};

// Sub-components for flexible composition
export const ModalHeader: Component<{ children: JSX.Element; class?: string }> = (props) => (
  <header class={cn("ui-modal-header", props.class)}>{props.children}</header>
);

export const ModalBody: Component<{ children: JSX.Element; class?: string }> = (props) => (
  <div class={cn("ui-modal-body", props.class)}>{props.children}</div>
);

export const ModalFooter: Component<{ children: JSX.Element; class?: string }> = (props) => (
  <footer class={cn("ui-modal-footer", props.class)}>{props.children}</footer>
);

// --- Specialized Modals ---



export interface ConfirmModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: () => void;
  title: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  kind?: "danger" | "warning" | "info";
  size?: ModalSize;
  children?: JSX.Element;
}

/**
 * Confirmation modal for destructive or important actions.
 */
export const ConfirmModal: Component<ConfirmModalProps> = (props) => {
  const handleConfirm = () => {
    props.onConfirm();
    props.onClose();
  };

  return (
    <Modal
      isOpen={props.isOpen}
      onClose={props.onClose}
      title={props.title}
      size={props.size || "sm"}
      footer={
        <>
          <Button variant="secondary" onClick={props.onClose}>
            {props.cancelText || "Cancel"}
          </Button>
          <Button
            variant={props.kind === "danger" ? "destructive" : "primary"}
            onClick={handleConfirm}
          >
            {props.confirmText || "Confirm"}
          </Button>
        </>
      }
    >
      <Show when={props.children} fallback={<p>{props.message}</p>}>
        {props.children}
      </Show>
    </Modal>
  );
};
