import { Component, JSX, createSignal, createEffect } from "solid-js";
import { ReferenceImage } from "./ReferenceImage";
import { assetDragSource, dndRegistry, currentDragItem, setDropTargetId, currentDropTargetId } from "../../../core/dnd";

/**
 * AssetCard Props - Pure Component Interface
 *
 * This component receives ALL data via props, making it suitable for
 * virtualization where items may be recycled.
 */
export interface AssetCardProps {
  // Identity
  id: number;
  filename: string;
  path: string;

  // Display
  thumbnailPath: string | null;
  width: number | null;
  height: number | null;

  // State (controlled externally)
  isSelected: boolean;
  isFocused?: boolean;
  style: JSX.CSSProperties;
  className?: string;

  // Callbacks (lifted to parent)
  onSelect: (id: number, multi: boolean) => void;
  onOpen: (id: number) => void;
  onContextMenu?: (e: MouseEvent, id: number) => void;

  // DnD Support - simplified params for drag source only
  getSelectedIds: () => (number | string)[];
  getItemInfo: (id: number) => { path: string; thumbnail_path: string | null } | undefined;
}

// Register directive for this file
assetDragSource;

/**
 * AssetCard - Pure Presentational Component
 *
 * Displays a single asset card with thumbnail. This component has NO internal
 * hooks - all state and actions come from props.
 *
 * DnD: Uses assetDragSource for dragging images.
 * Also accepts drops from tags (Tag-to-Image drop).
 */
export const AssetCard: Component<AssetCardProps> = (props) => {
  const [dragCounter, setDragCounter] = createSignal(0);
  let ref: HTMLDivElement | undefined;

  // Sync native focus when virtual focus changes
  createEffect(() => {
    if (props.isFocused && ref) {
      ref.focus({ preventScroll: true });
    }
  });

  // Check if this card is the current drop target
  const isDropTarget = () => currentDropTargetId() === props.id;

  // Handle tag drops onto this image
  const handleDragEnter = (e: DragEvent) => {
    e.preventDefault();
    const counter = dragCounter() + 1;
    setDragCounter(counter);

    if (counter === 1) {
      const dragging = currentDragItem();
      // Only accept TAG drops (not IMAGE drops)
      if (dragging?.type === "TAG") {
        // Use IMAGE strategy - that's the strategy for DROP TARGETS that are images
        const strategy = dndRegistry.get("IMAGE");
        if (strategy?.onDragOver?.(dragging)) {
          setDropTargetId(props.id);
        }
      }
    }
  };

  const handleDragOver = (e: DragEvent) => {
    e.preventDefault();
    const dragging = currentDragItem();
    if (dragging?.type === "TAG") {
      e.dataTransfer!.dropEffect = "copy";
      // Ensure target is set if enter was missed
      if (dragCounter() > 0 && currentDropTargetId() !== props.id) {
        setDropTargetId(props.id);
      }
    }
  };

  const handleDragLeave = () => {
    const counter = Math.max(0, dragCounter() - 1);
    setDragCounter(counter);
    if (counter === 0 && currentDropTargetId() === props.id) {
      setDropTargetId(null);
    }
  };

  const handleDrop = async (e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDragCounter(0);
    setDropTargetId(null);

    try {
      const json = e.dataTransfer?.getData("application/json");
      if (json) {
        const data = JSON.parse(json);
        // Accept TAG drops - use IMAGE strategy (strategy for image drop targets)
        if (data.type === "TAG") {
          const strategy = dndRegistry.get("IMAGE");
          if (strategy?.accepts(data)) {
            // The strategy handles selection internally via selectionState
            await strategy.onDrop(data, props.id);
          }
        }
      }
    } catch (err) {
      console.error("Drop failed", err);
    }
  };

  return (
    <div
      ref={ref}
      use:assetDragSource={{
        id: props.id,
        path: props.path,
        thumbnailPath: props.thumbnailPath,
        isSelected: props.isSelected,
        getSelectedIds: props.getSelectedIds,
        getItemInfo: props.getItemInfo,
      }}
      class={`virtual-item virtual-masonry-item ${props.isSelected ? "selected" : ""} ${props.isFocused ? "focused" : ""} ${isDropTarget() ? "drop-target-active" : ""} ${props.className || ""}`}
      style={props.style}
      // Accessibility
      role="gridcell"
      aria-selected={props.isSelected}
      aria-label={`Image: ${props.filename}`}
      tabIndex={props.isFocused ? 0 : -1}
      // Events
      onClick={(e) => {
        e.stopPropagation();
        props.onSelect(props.id, e.metaKey || e.ctrlKey);
      }}
      onDblClick={() => props.onOpen(props.id)}
      onContextMenu={(e) => props.onContextMenu?.(e, props.id)}
      // Drop handlers for Tag-to-Image
      onDragEnter={handleDragEnter}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      <div style={{ width: "100%", height: "100%", "pointer-events": "none" }}>
        <ReferenceImage
          id={props.id}
          src={props.path}
          thumbnail={props.thumbnailPath}
          alt={props.filename}
          width={props.width}
          height={props.height}
        />

        <div class="item-overlay">
          <span class="item-name">
            #{props.id} - {props.filename}
          </span>
        </div>
      </div>
    </div>
  );
};
