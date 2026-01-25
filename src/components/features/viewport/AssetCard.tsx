import { Component, JSX } from "solid-js";
import { ReferenceImage } from "./ReferenceImage";
import { type ImageItem } from "../../../utils/masonryLayout";

interface AssetCardProps {
  item: ImageItem;
  style: JSX.CSSProperties | undefined;
  className?: string;
  onClick?: (e: MouseEvent) => void;
  onContextMenu?: (e: MouseEvent) => void;
}

export const AssetCard: Component<AssetCardProps> = (props) => {
  return (
    <div
      class={`virtual-item virtual-masonry-item ${props.className || ""}`}
      style={props.style}
      onClick={props.onClick}
      onContextMenu={props.onContextMenu}
    >
      <ReferenceImage
        id={props.item.id}
        src={props.item.path}
        thumbnail={props.item.thumbnail_path}
        alt={props.item.filename}
        width={props.item.width}
        height={props.item.height}
      />
      
      {/* Overlay: Can be extracted further if complex */}
      <div class="item-overlay">
        <span class="item-name">#{props.item.id} - {props.item.filename}</span>
      </div>
    </div>
  );
};
