import { 
  Component, 
  JSX, 
  createSignal, 
  splitProps,
  Show,
  createEffect,
  onCleanup,
} from "solid-js";
import { Portal } from "solid-js/web";
import { cn } from "../../lib/utils";
import { createId } from "../../lib/primitives/createId";
import "./tooltip.css";

export type TooltipPosition = "top" | "bottom" | "left" | "right";

export interface TooltipProps {
  /** Content to display in the tooltip */
  content: string | JSX.Element;
  /** The trigger element */
  children: JSX.Element;
  /** Position of the tooltip relative to the trigger */
  position?: TooltipPosition;
  /** Delay in ms before showing (default: 200) */
  delay?: number;
  /** Whether the tooltip is disabled */
  disabled?: boolean;
  /** Additional class for the tooltip */
  class?: string;
}

/**
 * Tooltip component for displaying contextual information on hover.
 * 
 * @example
 * <Tooltip content="Settings">
 *   <button>⚙️</button>
 * </Tooltip>
 * 
 * @example
 * <Tooltip content="Copy to clipboard" position="bottom">
 *   <Button variant="icon">📋</Button>
 * </Tooltip>
 */
export const Tooltip: Component<TooltipProps> = (props) => {
  const [local] = splitProps(props, [
    "content",
    "children",
    "position",
    "delay",
    "disabled",
    "class",
  ]);

  const [isVisible, setIsVisible] = createSignal(false);
  const [coords, setCoords] = createSignal({ x: 0, y: 0 });
  
  let triggerRef: HTMLDivElement | undefined;
  let tooltipRef: HTMLDivElement | undefined;
  let showTimeout: ReturnType<typeof setTimeout> | undefined;
  
  const id = createId("tooltip");
  const position = () => local.position || "top";
  const delay = () => local.delay ?? 200;

  const calculatePosition = () => {
    if (!triggerRef || !tooltipRef) return;
    
    const triggerRect = triggerRef.getBoundingClientRect();
    const tooltipRect = tooltipRef.getBoundingClientRect();
    const scrollX = window.scrollX;
    const scrollY = window.scrollY;
    const gap = 8;

    let x = 0;
    let y = 0;

    switch (position()) {
      case "top":
        x = triggerRect.left + (triggerRect.width - tooltipRect.width) / 2 + scrollX;
        y = triggerRect.top - tooltipRect.height - gap + scrollY;
        break;
      case "bottom":
        x = triggerRect.left + (triggerRect.width - tooltipRect.width) / 2 + scrollX;
        y = triggerRect.bottom + gap + scrollY;
        break;
      case "left":
        x = triggerRect.left - tooltipRect.width - gap + scrollX;
        y = triggerRect.top + (triggerRect.height - tooltipRect.height) / 2 + scrollY;
        break;
      case "right":
        x = triggerRect.right + gap + scrollX;
        y = triggerRect.top + (triggerRect.height - tooltipRect.height) / 2 + scrollY;
        break;
    }

    // Viewport boundary checks
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;

    if (x < 0) x = gap;
    if (x + tooltipRect.width > viewportWidth) x = viewportWidth - tooltipRect.width - gap;
    if (y < 0) y = gap;
    if (y + tooltipRect.height > viewportHeight) y = viewportHeight - tooltipRect.height - gap;

    setCoords({ x, y });
  };

  const showTooltip = () => {
    if (local.disabled) return;
    
    showTimeout = setTimeout(() => {
      setIsVisible(true);
    }, delay());
  };

  const hideTooltip = () => {
    if (showTimeout) {
      clearTimeout(showTimeout);
      showTimeout = undefined;
    }
    setIsVisible(false);
  };

  // Calculate position after tooltip is rendered
  createEffect(() => {
    if (isVisible() && tooltipRef) {
      // Use requestAnimationFrame for accurate measurement
      requestAnimationFrame(calculatePosition);
    }
  });

  onCleanup(() => {
    if (showTimeout) clearTimeout(showTimeout);
  });

  return (
    <>
      <div
        ref={triggerRef}
        onMouseEnter={showTooltip}
        onMouseLeave={hideTooltip}
        onFocus={showTooltip}
        onBlur={hideTooltip}
        aria-describedby={isVisible() ? id : undefined}
        class="ui-tooltip-trigger"
      >
        {local.children}
      </div>

      <Show when={isVisible()}>
        <Portal>
          <div
            ref={tooltipRef}
            id={id}
            role="tooltip"
            class={cn(
              "ui-tooltip",
              `ui-tooltip-${position()}`,
              local.class
            )}
            style={{
              top: `${coords().y}px`,
              left: `${coords().x}px`,
            }}
          >
            {local.content}
          </div>
        </Portal>
      </Show>
    </>
  );
};
