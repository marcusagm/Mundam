import { Accessor, createEffect, onCleanup } from "solid-js";

const FOCUSABLE_SELECTOR = [
  'a[href]',
  'area[href]',
  'input:not([disabled]):not([type="hidden"])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  'button:not([disabled])',
  'iframe',
  'object',
  'embed',
  '[contenteditable]',
  '[tabindex]:not([tabindex="-1"])',
].join(",");

/**
 * Creates a focus trap within the given element.
 * Prevents focus from leaving the element via Tab key.
 * 
 * @param ref - Accessor returning the container element
 * @param active - Whether the trap is active
 */
export function createFocusTrap(
  ref: Accessor<HTMLElement | undefined>,
  active: Accessor<boolean>
): void {
  createEffect(() => {
    const el = ref();
    if (!active() || !el) return;

    const getFocusableElements = () => {
      return Array.from(el.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR))
        .filter(el => el.offsetParent !== null); // Filter hidden elements
    };

    // Focus first element on activation
    const focusables = getFocusableElements();
    if (focusables.length > 0) {
      focusables[0].focus();
    }

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key !== "Tab") return;

      const elements = getFocusableElements();
      if (elements.length === 0) return;

      const first = elements[0];
      const last = elements[elements.length - 1];

      if (e.shiftKey) {
        // Shift + Tab: going backwards
        if (document.activeElement === first) {
          e.preventDefault();
          last.focus();
        }
      } else {
        // Tab: going forward
        if (document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    };

    el.addEventListener("keydown", handleKeyDown);

    onCleanup(() => {
      el.removeEventListener("keydown", handleKeyDown);
    });
  });
}
