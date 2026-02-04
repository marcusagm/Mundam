import { 
  Component, 
  Show, 
  createEffect, 
  onCleanup, 
  splitProps,
  createSignal,
  createMemo
} from "solid-js";
import { Portal } from "solid-js/web";
import { X } from "lucide-solid";

import { createFocusTrap } from "../../lib/primitives";
import { createId } from "../../lib/primitives/createId";
import { useShortcuts, createConditionalScope } from "../../core/input";
import { Button } from "./Button";
import { Input } from "./Input";
import "./prompt-modal.css";

export interface PromptModalProps {
  /** Whether the modal is open */
  isOpen: boolean;
  /** Callback when modal should close */
  onClose: () => void;
  /** Callback when the user confirms the input */
  onConfirm: (value: string) => void;
  /** Modal title */
  title: string;
  /** Description text displayed above the input */
  description?: string;
  /** Initial value of the input */
  initialValue?: string;
  /** Placeholder text for the input */
  placeholder?: string;
  /** Text for the confirm button */
  confirmText?: string;
  /** Text for the cancel button */
  cancelText?: string;
  /** 
   * Validation function. Returns an error message string if invalid, or undefined/null if valid.
   */
  validate?: (value: string) => string | undefined | null;
  /** External error message to display */
  errorMessage?: string;
  /** Whether the input is required */
  required?: boolean;
}

/**
 * PromptModal component for capturing user input with validation.
 * 
 * Implemented as a standalone component for maximum robustness and accessibility,
 * wrapping the entire dialog in a form element to ensure native submission behavior.
 */
export const PromptModal: Component<PromptModalProps> = (props) => {
  const [local] = splitProps(props, [
    "isOpen",
    "onClose",
    "onConfirm",
    "title",
    "description",
    "initialValue",
    "placeholder",
    "confirmText",
    "cancelText",
    "validate",
    "errorMessage",
    "required"
  ]);

  let containerRef: HTMLFormElement | undefined;
  let inputRef: HTMLInputElement | undefined;

  const [value, setValue] = createSignal(local.initialValue || "");
  const [internalError, setInternalError] = createSignal<string | null>(null);
  
  // Sync internal value with initialValue when reopening
  createEffect(() => {
    if (local.isOpen) {
      setValue(local.initialValue || "");
      setInternalError(null);
    }
  });

  const error = createMemo(() => local.errorMessage || internalError());

  const titleId = createId("prompt-modal-title");
  const descId = createId("prompt-modal-desc");

  // Focus trap
  createFocusTrap(() => containerRef, () => local.isOpen);

  // Input System Integration
  createConditionalScope("prompt-modal", () => local.isOpen, undefined, true);
  
  useShortcuts([
    {
      keys: "Escape",
      name: "Close Prompt",
      scope: "prompt-modal",
      enabled: () => local.isOpen,
      action: () => {
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

  // Focus the input when opening
  createEffect(() => {
    if (local.isOpen && inputRef) {
        // Short timeout to ensure Modal is mounted and visible
        setTimeout(() => inputRef?.focus(), 50);
    }
  });

  const handleSubmit = (e: Event) => {
    e.preventDefault();
    
    // Validate
    if (local.required && !value().trim()) {
        setInternalError("This field is required.");
        return;
    }

    if (local.validate) {
        const validationError = local.validate(value());
        if (validationError) {
            setInternalError(validationError);
            return;
        }
    }

    local.onConfirm(value());
    local.onClose();
  };

  const handleOverlayClick = (e: MouseEvent) => {
    if (e.target === e.currentTarget) {
      local.onClose();
    }
  };

  return (
    <Show when={local.isOpen}>
      <Portal>
        <div
          class="ui-prompt-modal-overlay"
          onClick={handleOverlayClick}
          aria-hidden="true"
        />
        <form
          ref={containerRef}
          onSubmit={handleSubmit}
          class="ui-prompt-modal-container"
          role="dialog"
          aria-modal="true"
          aria-labelledby={titleId}
          aria-describedby={local.description ? descId : undefined}
          tabindex={-1}
          novalidate
        >
            <header class="ui-prompt-modal-header">
                <h2 id={titleId} class="ui-prompt-modal-title">
                  {local.title}
                </h2>
                <button
                  type="button"
                  class="ui-prompt-modal-close"
                  onClick={local.onClose}
                  aria-label="Close modal"
                >
                  <X size={18} />
                </button>
            </header>

            <div class="ui-prompt-modal-body">
                <Show when={local.description}>
                    <p id={descId} class="ui-prompt-modal-description">{local.description}</p>
                </Show>
                
                <Input
                    ref={inputRef}
                    value={value()}
                    onInput={(e) => {
                        setValue(e.currentTarget.value);
                        if (internalError()) setInternalError(null);
                    }}
                    placeholder={local.placeholder}
                    error={!!error()}
                    errorMessage={error() || undefined}
                    required={local.required}
                />
            </div>

            <footer class="ui-prompt-modal-footer">
               <Button type="button" variant="ghost" onClick={local.onClose}>
                {local.cancelText || "Cancel"}
              </Button>
              <Button type="submit" variant="primary">
                {local.confirmText || "Confirm"}
              </Button>
            </footer>
        </form>
      </Portal>
    </Show>
  );
};
