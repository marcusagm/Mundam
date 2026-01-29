import { Component } from "solid-js";
import { type ImageItem } from "../../../types";
import { CommonMetadata } from "./CommonMetadata";
import { ImageMetadata } from "./ImageMetadata";
import { InspectorTags } from "./InspectorTags";
import { AdvancedMetadata } from "./AdvancedMetadata";
import { Accordion } from "../../ui/Accordion";
import "./inspector.css";

interface ImageInspectorProps {
    item: ImageItem;
}

export const ImageInspector: Component<ImageInspectorProps> = (props) => {
    return (
        <div class="inspector-content">
            {/* Preview */}
            <div class="inspector-preview square">
                <img 
                    class="preview-image"
                    src={props.item.thumbnail_path ? `thumb://localhost/${props.item.thumbnail_path.split(/[\\/]/).pop()}` : ""} 
                    alt={props.item.filename}
                />
            </div>

            {/* Accordions */}
            <Accordion>
                <CommonMetadata item={props.item} />
                <ImageMetadata item={props.item} />
                <InspectorTags />
                <AdvancedMetadata item={props.item} />
            </Accordion>
        </div>
    );
};
