import { Component, splitProps, Show, createMemo } from "solid-js";
import { cn } from "../../lib/utils";
import { createId } from "../../lib/primitives/createId";
import "./progress-bar.css";

export type ProgressBarSize = "sm" | "md" | "lg";

export interface ProgressBarProps {
  /** Current value (0-100 or current if max is specified) */
  value: number;
  /** Maximum value (default: 100) */
  max?: number;
  /** Size variant */
  size?: ProgressBarSize;
  /** Whether to show the percentage label */
  showLabel?: boolean;
  /** Custom label text (shown before percentage) */
  label?: string;
  /** Indeterminate state (loading animation) */
  indeterminate?: boolean;
  /** Additional CSS class */
  class?: string;
}

/**
 * ProgressBar component for indicating progress or loading state.
 * 
 * @example
 * // Basic usage
 * <ProgressBar value={50} />
 * 
 * @example
 * // With label
 * <ProgressBar value={75} showLabel label="Uploading..." />
 * 
 * @example
 * // Indeterminate loading
 * <ProgressBar indeterminate />
 */
export const ProgressBar: Component<ProgressBarProps> = (props) => {
  const [local] = splitProps(props, [
    "value",
    "max",
    "size",
    "showLabel",
    "label",
    "indeterminate",
    "class",
  ]);

  const id = createId("progress");
  const labelId = `${id}-label`;
  
  const max = () => local.max || 100;
  const size = () => local.size || "sm";
  
  const percentage = createMemo(() => {
    const val = Math.max(0, Math.min(local.value, max()));
    return (val / max()) * 100;
  });

  return (
    <div
      class={cn("ui-progress-container", local.class)}
      role="progressbar"
      aria-valuenow={local.indeterminate ? undefined : local.value}
      aria-valuemin={0}
      aria-valuemax={max()}
      aria-labelledby={local.label ? labelId : undefined}
      aria-busy={local.indeterminate}
    >
      <Show when={local.showLabel}>
        <div class="ui-progress-label">
          <Show when={local.label}>
            <span id={labelId}>{local.label}</span>
          </Show>
          <Show when={!local.indeterminate}>
            <span class="ui-progress-percentage">{Math.round(percentage())}%</span>
          </Show>
        </div>
      </Show>

      <div class={cn("ui-progress-track", `ui-progress-${size()}`)}>
        <div
          class={cn(
            "ui-progress-fill",
            local.indeterminate && "ui-progress-indeterminate"
          )}
          style={local.indeterminate ? undefined : { width: `${percentage()}%` }}
        />
      </div>
    </div>
  );
};
