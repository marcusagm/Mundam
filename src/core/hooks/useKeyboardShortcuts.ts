import { onMount, onCleanup } from "solid-js";
import { useSelection, useLibrary } from "./index";

export function useKeyboardShortcuts() {
  const selection = useSelection();
  const lib = useLibrary();

  const handleKeyDown = (e: KeyboardEvent) => {
    // Ignore input fields
    if (["INPUT", "TEXTAREA"].includes((e.target as HTMLElement).tagName)) {
      if (e.key === "Escape") {
        (e.target as HTMLElement).blur();
      }
      return;
    }

    // Ctrl/Cmd + A (Select All)
    if ((e.metaKey || e.ctrlKey) && e.key === "a") {
      e.preventDefault();
      const allIds = lib.items.map(i => i.id);
      selection.select(allIds);
      return;
    }

    // Escape (Deselect All)
    if (e.key === "Escape") {
      selection.select([]);
      return;
    }
    
    // Ctrl/Cmd + F (Focus Search)
    if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        const searchInput = document.querySelector(".shell-header input") as HTMLInputElement;
        if (searchInput) searchInput.focus();
    }
  };

  onMount(() => {
    window.addEventListener("keydown", handleKeyDown);
  });

  onCleanup(() => {
    window.removeEventListener("keydown", handleKeyDown);
  });
}
