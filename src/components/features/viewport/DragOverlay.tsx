/**
 * DragOverlay Component
 * 
 * Visual indicator for drag-and-drop operations in virtualized viewports.
 * Shows a line indicating where the dropped item will be placed.
 */

import { Component, Show } from "solid-js";
import "./DragOverlay.css";

export interface DragOverlayProps {
  /** Whether the overlay is visible */
  visible: boolean;
  /** Y position of the indicator line (in container coordinates) */
  top: number;
  /** X position of the indicator line */
  left: number;
  /** Width of the indicator line */
  width: number;
  /** Current scroll offset (to position correctly) */
  scrollTop: number;
}

/**
 * Drop indicator line component.
 * Shows a horizontal line where the dragged item will be placed.
 */
export const DragOverlay: Component<DragOverlayProps> = (props) => {
  return (
    <Show when={props.visible}>
      <div
        class="drag-overlay-indicator"
        style={{
          position: "absolute",
          top: `${props.top - props.scrollTop}px`,
          left: `${props.left}px`,
          width: `${props.width}px`,
          height: "4px",
          "background-color": "var(--color-accent, #3b82f6)",
          "border-radius": "2px",
          "pointer-events": "none",
          "z-index": 1000,
          "box-shadow": "0 0 8px var(--color-accent, #3b82f6)",
          transition: "top 0.1s ease-out, left 0.1s ease-out",
        }}
      />
    </Show>
  );
};
