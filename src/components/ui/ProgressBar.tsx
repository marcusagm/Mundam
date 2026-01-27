import { Component, Show } from "solid-js";
import "./progress-bar.css";

interface ProgressBarProps {
  value: number; // 0 to 100 or current value if total is provided
  max?: number;
  size?: "sm" | "md" | "lg";
  showLabel?: boolean;
  label?: string; // Custom label text "Processing..."
  class?: string;
}

export const ProgressBar: Component<ProgressBarProps> = (props) => {
  const percentage = () => {
    const max = props.max || 100;
    const val = Math.max(0, Math.min(props.value, max));
    return (val / max) * 100;
  };

  return (
    <div class={`progress-container ${props.class || ""}`}>
      <Show when={props.showLabel}>
        <div class="progress-label">
           <span>{props.label}</span>
           <span>{Math.round(percentage())}%</span>
        </div>
      </Show>
      
      <div class={`progress-track ${props.size || "sm"}`}>
        <div 
            class="progress-fill" 
            style={{ width: `${percentage()}%` }} 
            role="progressbar" 
            aria-valuenow={props.value} 
            aria-valuemin={0} 
            aria-valuemax={props.max || 100}
        />
      </div>
    </div>
  );
};
