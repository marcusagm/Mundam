import { onMount, onCleanup } from "solid-js";
import { appActions, useAppStore } from "../store/appStore";

export function useKeyboardShortcuts() {
  const { state } = useAppStore();

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
      const allIds = state.items.map(i => i.id);
      appActions.setSelection(allIds);
      return;
    }

    // Escape (Deselect All)
    if (e.key === "Escape") {
      appActions.setSelection([]);
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
