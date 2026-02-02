/**
 * Asset Drag Source Directive
 * 
 * Simplified directive that ONLY handles drag start and ghost creation.
 * Drop handling is now done at the container level via ViewportDropZone.
 * 
 * This separation allows virtualization to work correctly since we don't
 * need to attach drop handlers to each item.
 */

import { onCleanup } from "solid-js";
import { setDragItem } from "./dnd-core";
import { createDragGhost } from "./ghost";

export interface AssetDragSourceParams {
  /** Unique ID of this item */
  id: number;
  /** Path to the asset file */
  path: string;
  /** Thumbnail path for ghost image */
  thumbnailPath: string | null;
  /** Whether this item is currently selected */
  isSelected: boolean;
  /** Get all currently selected IDs (for multi-drag) */
  getSelectedIds: () => (number | string)[];
  /** Get item info by ID (for ghost creation) */
  getItemInfo: (id: number) => { path: string; thumbnail_path: string | null } | undefined;
}

/**
 * SolidJS Directive for Asset Drag Source.
 * Only handles dragstart and dragend - no drop target logic.
 */
export function assetDragSource(el: HTMLElement, accessor: () => AssetDragSourceParams) {
  const handleDragStart = (e: DragEvent) => {
    e.stopPropagation();
    if (!e.dataTransfer) return;

    const params = accessor();
    const { id, isSelected, getSelectedIds, getItemInfo } = params;

    // Determine which IDs to drag
    let ids: number[] = [id];
    if (isSelected) {
      const selectedIds = getSelectedIds();
      if (selectedIds.includes(id)) {
        ids = selectedIds.filter((sid): sid is number => typeof sid === "number");
      }
    }

    // Set drag data
    const data = { type: "IMAGE", payload: { ids } };
    setDragItem(data as any);

    e.dataTransfer.effectAllowed = "copyMove";
    e.dataTransfer.setData("application/json", JSON.stringify(data));

    // Build paths for external drag
    const validPaths: string[] = [];
    const cleanPaths: string[] = [];

    ids.forEach((itemId) => {
      const info = getItemInfo(itemId);
      if (info?.path) {
        validPaths.push(`file://${info.path}`);
        cleanPaths.push(info.path);
      }
    });

    if (validPaths.length > 0) {
      e.dataTransfer.setData("text/uri-list", validPaths.join("\r\n"));
      e.dataTransfer.setData("text/plain", cleanPaths.join("\n"));
    }

    // Create ghost image
    const ghostItems = ids.map((itemId) => {
      const info = getItemInfo(itemId);
      return {
        id: itemId,
        path: info?.path || "",
        thumbnail_path: info?.thumbnail_path || null,
      };
    });

    const ghost = createDragGhost(ghostItems as any);
    e.dataTransfer.setDragImage(ghost, 0, 0);

    // Cleanup ghost after render
    setTimeout(() => {
      if (ghost.parentNode) {
        document.body.removeChild(ghost);
      }
    }, 0);
  };

  const handleDragEnd = () => {
    setDragItem(null);
  };

  // Attach listeners
  el.setAttribute("draggable", "true");
  el.addEventListener("dragstart", handleDragStart);
  el.addEventListener("dragend", handleDragEnd);

  onCleanup(() => {
    el.removeEventListener("dragstart", handleDragStart);
    el.removeEventListener("dragend", handleDragEnd);
  });
}

// TypeScript directive declaration
declare module "solid-js" {
  namespace JSX {
    interface Directives {
      assetDragSource: AssetDragSourceParams;
    }
  }
}
