import { Component } from "solid-js";
import { type ImageItem } from "../../../types";
import { Accordion } from "../../ui/Accordion";
import { InspectorTags } from "./InspectorTags";
import { CommonMetadata } from "./CommonMetadata";
import "./inspector.css";

interface AudioInspectorProps {
    item: ImageItem;
}

export const AudioInspector: Component<AudioInspectorProps> = (props) => {
    // Protocol `asset://` converts local path to browser-readable URL
    // Requires secure context or custom protocol handler configuration in Tauri
    const assetUrl = () => `asset://localhost/${props.item.path}`;

    return (
        <div class="inspector-content">
             <div class="inspector-preview" style="height: auto; aspect-ratio: unset; padding: 24px;">
                <audio 
                    controls 
                    class="inspector-audio-player" 
                    src={assetUrl()} 
                >
                    Your browser does not support the audio element.
                </audio>
            </div>
            
            <Accordion>
                 <CommonMetadata item={props.item} />
                 <InspectorTags />
            </Accordion>
        </div>
    );
};
