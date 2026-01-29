import { Component, JSX, createSignal } from "solid-js";
import { ReferenceImage } from "./ReferenceImage";
import { type ImageItem } from "../../../types";
import { dndRegistry, setDragItem, currentDragItem } from "../../../core/dnd";
import { useLibrary, useSelection, useViewport } from "../../../core/hooks";

interface AssetCardProps {
  item: ImageItem;
  style: JSX.CSSProperties | undefined;
  className?: string;
  selected?: boolean;
  onClick?: (e: MouseEvent) => void;
  onContextMenu?: (e: MouseEvent) => void;
}

export const AssetCard: Component<AssetCardProps> = (props) => {
  const lib = useLibrary();
  const selection = useSelection();
  const [isDropTarget, setIsDropTarget] = createSignal(false);
  
  // Helper to create custom ghost
  const createDragGhost = (items: ImageItem[]) => {
      const container = document.createElement("div");
      
      const count = items.length;
      
      Object.assign(container.style, {
          position: "absolute",
          top: "-1000px",
          left: "-1000px",
          width: "120px", // Smaller
          height: "auto",
          zIndex: "10000"
      });
      
      // Deck Effect
      // We render up to 3 items stacked
      const previewItems = items.slice(0, 3).reverse(); // Reverse so first is on top
      
      previewItems.forEach((item, index) => {
          const card = document.createElement("div");
          const isTop = index === previewItems.length - 1;
          
          Object.assign(card.style, {
              position: isTop ? "relative" : "absolute",
              top: isTop ? "0" : `${(previewItems.length - 1 - index) * 4}px`,
              left: isTop ? "0" : `${(previewItems.length - 1 - index) * 4}px`,
              width: "100%",
              height: "80px",
              background: "var(--bg-surface)",
              border: "1px solid var(--border-active)",
              borderRadius: "4px",
              boxShadow: "0 4px 8px rgba(0,0,0,0.4)",
              overflow: "hidden",
              transform: count > 1 ? `rotate(${(previewItems.length - 1 - index) * 2 - 2}deg)` : "none",
              zIndex: index
          });
          
          if (item.thumbnail_path) {
             const img = document.createElement("img");
             const filename = item.thumbnail_path.split(/[\\/]/).pop();
             img.src = `thumb://localhost/${filename}`;
             
             Object.assign(img.style, {
                 width: "100%",
                 height: "100%",
                 objectFit: "cover",
                 display: "block"
             });
             card.appendChild(img);
          }
          
          container.appendChild(card);
      });
      
      // Badge if > 1
      if (count > 1) {
          const badge = document.createElement("div");
          Object.assign(badge.style, {
              position: "absolute",
              top: "-6px",
              right: "-6px",
              background: "var(--accent-color)",
              color: "#fff",
              fontSize: "10px",
              fontWeight: "bold",
              padding: "2px 6px",
              borderRadius: "10px",
              zIndex: "100",
              boxShadow: "0 2px 4px rgba(0,0,0,0.3)"
          });
          badge.innerText = String(count);
          container.appendChild(badge);
      }
      
      document.body.appendChild(container);
      return container;
  };

  const handleDragStart = (e: DragEvent) => {
    e.stopPropagation();
    if (!e.dataTransfer) return;
    
    // Determine selection
    let ids = [props.item.id];
    const selectedIds = selection.selectedIds;
    
    // Check if clicked item is part of selection
    if (props.selected && selectedIds.includes(props.item.id)) {
        ids = [...selectedIds];
    }
    
    const data = { type: "IMAGE", payload: { ids } };
    setDragItem(data as any);
    
    e.dataTransfer.effectAllowed = "copyMove";
    e.dataTransfer.setData("application/json", JSON.stringify(data));
    
    // 1. Gather Items for Ghost and Paths
    const draggedItems: ImageItem[] = [];
    const validPaths: string[] = [];
    const cleanPaths: string[] = [];
    
    ids.forEach(id => {
        const item = lib.items.find(i => i.id === id);
        if (item) {
            draggedItems.push(item);
            if (item.path) {
                validPaths.push(`file://${item.path}`);
                cleanPaths.push(item.path);
            }
        }
    });

    // 2. Set URI List (Multi-file support)
    if (validPaths.length > 0) {
        e.dataTransfer.setData("text/uri-list", validPaths.join("\r\n"));
        // Some apps fallback to text/plain for paths
        e.dataTransfer.setData("text/plain", cleanPaths.join("\n"));
    }

    // 3. Custom Ghost
    const ghost = createDragGhost(draggedItems);
    e.dataTransfer.setDragImage(ghost, 0, 0);
    
    // Cleanup ghost after browser snapshots it
    setTimeout(() => {
        document.body.removeChild(ghost);
    }, 0);
  };
  
  const handleDragEnd = () => {
      setDragItem(null);
      setIsDropTarget(false);
  };

  const handleDragEnter = (e: DragEvent) => {
      e.preventDefault();
  };

  const handleDragOver = (e: DragEvent) => {
      e.preventDefault(); 
      const strategy = dndRegistry.get("IMAGE");
      const item = currentDragItem();
      if (strategy && strategy.onDragOver && item && strategy.onDragOver(item)) {
           e.dataTransfer!.dropEffect = "copy";
           setIsDropTarget(true);
      }
  };
  
  const handleDragLeave = () => setIsDropTarget(false);

  const handleDrop = async (e: DragEvent) => {
      e.preventDefault();
      e.stopPropagation();
      setIsDropTarget(false);
      
      try {
          const json = e.dataTransfer?.getData("application/json");
          if (json) {
              const item = JSON.parse(json);
              const strategy = dndRegistry.get("IMAGE");
              if (strategy && strategy.accepts(item)) {
                  await strategy.onDrop(item, props.item.id);
              }
          }
      } catch (err) {
          console.error("Drop failed", err);
      }
  };

    const viewport = useViewport();

    return (
    <div
      class={`virtual-item virtual-masonry-item ${props.selected ? "selected" : ""} ${props.className || ""} ${isDropTarget() ? "drop-target-active" : ""}`}
      style={{
          ...props.style
      }}
      draggable={true}
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
      onDragEnter={handleDragEnter}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      onClick={props.onClick}
      onDblClick={() => viewport.openItem(props.item.id.toString())}
      onContextMenu={props.onContextMenu}
    >
        <div style={{ width: "100%", height: "100%", "pointer-events": "none" }}>
          <ReferenceImage
            id={props.item.id}
            src={props.item.path}
            thumbnail={props.item.thumbnail_path}
            alt={props.item.filename}
            width={props.item.width}
            height={props.item.height}
          />
          
          <div class="item-overlay">
            <span class="item-name">#{props.item.id} - {props.item.filename}</span>
          </div>
        </div>
    </div>
  );
};
