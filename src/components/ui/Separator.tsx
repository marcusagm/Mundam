import { Component, JSX, splitProps } from "solid-js";
import { cn } from "../../lib/utils";
import "./separator.css";

export type SeparatorOrientation = "horizontal" | "vertical";

export interface SeparatorProps extends JSX.HTMLAttributes<HTMLDivElement> {
  /** Orientation of the separator */
  orientation?: SeparatorOrientation;
  /** Whether this is decorative (default: true). If false, separator will be exposed to screen readers. */
  decorative?: boolean;
}

/**
 * Separator component for visually dividing content.
 * 
 * @example
 * <div>
 *   <p>First section</p>
 *   <Separator />
 *   <p>Second section</p>
 * </div>
 * 
 * @example
 * <div style={{ display: "flex", gap: "8px" }}>
 *   <span>Item 1</span>
 *   <Separator orientation="vertical" />
 *   <span>Item 2</span>
 * </div>
 */
export const Separator: Component<SeparatorProps> = (props) => {
  const [local, others] = splitProps(props, [
    "orientation",
    "decorative",
    "class",
  ]);
  
  const orientation = () => local.orientation || "horizontal";
  const decorative = () => local.decorative ?? true;

  return (
    <div
      class={cn(
        "ui-separator",
        `ui-separator-${orientation()}`,
        local.class
      )}
      role={decorative() ? "none" : "separator"}
      aria-orientation={!decorative() ? orientation() : undefined}
      data-orientation={orientation()}
      {...others}
    />
  );
};
