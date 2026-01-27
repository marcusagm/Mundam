import { Component, Show } from "solid-js";
import { ProgressBar } from "./ProgressBar";
import "./loader.css";

interface LoaderProps {
  /**
   * Size of the spinner
   * @default "md"
   */
  size?: "sm" | "md" | "lg";
  
  /**
   * Whether to take up the full screen/container
   * @default false
   */
  fullscreen?: boolean;

  /**
   * Text to display below the spinner
   */
  text?: string;

  /**
   * If provided, shows a progress bar below the text
   * Value should be percentage (0-100) or current if max is set
   */
  progress?: number;
  
  /**
   * Max value for progress
   */
  max?: number;

  class?: string;
}

export const Loader: Component<LoaderProps> = (props) => {
  return (
    <div 
      class={`loader-container ${props.fullscreen ? "fullscreen" : ""} ${props.class || ""}`}
      role="status"
    >
      <div class={`loader-spinner ${props.size || "md"}`} />
      
      <Show when={props.text}>
        <span class="loader-text">{props.text}</span>
      </Show>

      <Show when={typeof props.progress === "number"}>
        <div class="loader-progress-wrapper">
             <ProgressBar 
                value={props.progress!} 
                max={props.max} 
                size="sm" 
             />
        </div>
      </Show>
    </div>
  );
};
