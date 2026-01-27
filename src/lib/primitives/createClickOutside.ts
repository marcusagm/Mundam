import { Accessor, onCleanup, onMount } from "solid-js";

/**
 * Creates a click/touch outside listener for the given element.
 * Useful for closing dropdowns, menus, and modals.
 * 
 * @param ref - Accessor returning the element to watch
 * @param handler - Callback when click outside occurs
 */
export function createClickOutside(
  ref: Accessor<HTMLElement | undefined>,
  handler: (event: MouseEvent | TouchEvent) => void
): void {
  const listener = (event: MouseEvent | TouchEvent) => {
    const el = ref();
    if (!el) return;
    
    const target = event.target as Node;
    
    // Check if click is inside the element
    if (el.contains(target)) return;
    
    handler(event);
  };

  onMount(() => {
    // Use mousedown instead of click for faster response
    document.addEventListener("mousedown", listener);
    document.addEventListener("touchstart", listener);

    onCleanup(() => {
      document.removeEventListener("mousedown", listener);
      document.removeEventListener("touchstart", listener);
    });
  });
}
