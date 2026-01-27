import { Component, createSignal, Show, splitProps } from "solid-js";
import { Portal } from "solid-js/web";
import { cn } from "../../lib/utils";
import "./count-badge.css";

export type CountBadgeVariant = "primary" | "secondary" | "outline";

export interface CountBadgeProps {
  /** The count value to display */
  count: number;
  /** Visual variant */
  variant?: CountBadgeVariant;
  /** Maximum count before showing "max+" (default: 99) */
  max?: number;
  /** Whether to show 0 count (default: false) */
  showZero?: boolean;
  /** Additional CSS class */
  class?: string;
}

/**
 * CountBadge for displaying numeric values with auto-formatting.
 * Shows abbreviated values (1k, 1M) with tooltip for exact count.
 * 
 * @example
 * <CountBadge count={1234} />
 * // Displays "1.2k" with tooltip "1,234"
 * 
 * @example
 * <CountBadge count={5} variant="primary" />
 */
export const CountBadge: Component<CountBadgeProps> = (props) => {
  const [local] = splitProps(props, [
    "count",
    "variant",
    "max",
    "showZero",
    "class",
  ]);

  const [showTooltip, setShowTooltip] = createSignal(false);
  const [coords, setCoords] = createSignal({ x: 0, y: 0 });
  
  let badgeRef: HTMLSpanElement | undefined;

  const variant = () => local.variant || "secondary";
  const max = () => local.max ?? 9999;
  const showZero = () => local.showZero ?? false;

  const shouldShow = () => local.count > 0 || showZero();

  const formatNumber = (num: number): string => {
    if (num > max()) {
      return `${formatNumber(max())}+`;
    }
    if (num >= 1000000) {
      return (num / 1000000).toFixed(1).replace(/\.0$/, "") + "M";
    }
    if (num >= 1000) {
      return (num / 1000).toFixed(1).replace(/\.0$/, "") + "k";
    }
    return num.toString();
  };

  const handleMouseEnter = () => {
    if (!badgeRef) return;
    
    const rect = badgeRef.getBoundingClientRect();
    setCoords({
      x: rect.left + rect.width / 2,
      y: rect.top - 8,
    });
    setShowTooltip(true);
  };

  return (
    <Show when={shouldShow()}>
      <span
        ref={badgeRef}
        class={cn(
          "ui-count-badge",
          `ui-count-badge-${variant()}`,
          local.class
        )}
        onMouseEnter={handleMouseEnter}
        onMouseLeave={() => setShowTooltip(false)}
        aria-label={`Count: ${local.count.toLocaleString()}`}
      >
        {formatNumber(local.count)}

        <Show when={showTooltip() && local.count >= 1000}>
          <Portal>
            <div
              class="ui-count-badge-tooltip"
              role="tooltip"
              style={{
                left: `${coords().x}px`,
                top: `${coords().y}px`,
              }}
            >
              {local.count.toLocaleString()}
              <div class="ui-count-badge-tooltip-arrow" />
            </div>
          </Portal>
        </Show>
      </span>
    </Show>
  );
};
