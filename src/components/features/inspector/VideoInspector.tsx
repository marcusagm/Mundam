import { Component } from "solid-js";
import { type ImageItem } from "../../../types";
import { Accordion } from "../../ui/Accordion";
import { InspectorTags } from "./InspectorTags";
import { CommonMetadata } from "./CommonMetadata";
import "./inspector.css";

interface VideoInspectorProps {
    item: ImageItem;
}

export const VideoInspector: Component<VideoInspectorProps> = (props) => {
    // Protocol `asset://` converts local path to browser-readable URL
    const assetUrl = () => `asset://localhost/${props.item.path}`;

    return (
        <div class="inspector-content">
             <div class="inspector-preview" style="height: auto; aspect-ratio: unset; padding: 0;">
                <video 
                    controls 
                    class="inspector-video-player" 
                    src={assetUrl()} 
                >
                    Your browser does not support the video element.
                </video>
            </div>
            
            <Accordion>
                 <CommonMetadata item={props.item} />
                 <InspectorTags />
            </Accordion>
        </div>
    );
};
