import { Component } from "solid-js";
import { Maximize2, Hash } from "lucide-solid";
import { AccordionItem } from "../../ui/Accordion";
import { type ImageItem } from "../../../types";
import "./inspector.css";

interface ImageMetadataProps {
    item: ImageItem | null;
}

export const ImageMetadata: Component<ImageMetadataProps> = (props) => {
    return (
        <AccordionItem 
            value="image-details" 
            title="Image Details" 
            defaultOpen
            icon={<Maximize2 size={14} />}
        >
             <div class="inspector-grid">
                <div class="inspector-meta-item">
                    <span class="inspector-meta-label">Dimensions</span>
                    <span class="inspector-meta-value">
                        {props.item?.width || "-"} x {props.item?.height || "-"}
                    </span>
                </div>
                 <div class="inspector-meta-item">
                    <span class="inspector-meta-label">Megapixels</span>
                    <span class="inspector-meta-value">
                        {props.item?.width && props.item?.height 
                            ? ((props.item.width * props.item.height) / 1000000).toFixed(1) + " MP"
                            : "-"}
                    </span>
                </div>
                <div class="inspector-meta-item">
                     <span class="inspector-meta-label">ID</span>
                     <span class="inspector-meta-value">
                        <Hash size={10} />
                        {props.item?.id}
                     </span>
                </div>
            </div>
        </AccordionItem>
    );
};
