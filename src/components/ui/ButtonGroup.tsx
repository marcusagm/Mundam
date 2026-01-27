import { Component, JSX, splitProps, createMemo } from "solid-js";
import { cn } from "../../lib/utils";
import "./button-group.css";

type ButtonGroupOrientation = "horizontal" | "vertical";
type ButtonGroupSize = "sm" | "md" | "lg";

export interface ButtonGroupProps extends JSX.HTMLAttributes<HTMLDivElement> {
  orientation?: ButtonGroupOrientation;
  size?: ButtonGroupSize;
  attached?: boolean;
  children: JSX.Element;
}

/**
 * ButtonGroup component for grouping related buttons.
 * Supports horizontal/vertical layouts and attached styling.
 * 
 * @example
 * <ButtonGroup attached>
 *   <Button variant="ghost">Left</Button>
 *   <Button variant="ghost">Center</Button>
 *   <Button variant="ghost">Right</Button>
 * </ButtonGroup>
 */
export const ButtonGroup: Component<ButtonGroupProps> = (props) => {
  const [local, others] = splitProps(props, [
    "class",
    "orientation",
    "size",
    "attached",
    "children",
  ]);

  const classes = createMemo(() =>
    cn(
      "ui-button-group",
      `ui-button-group-${local.orientation || "horizontal"}`,
      local.attached && "ui-button-group-attached",
      local.size && `ui-button-group-${local.size}`,
      local.class
    )
  );

  return (
    <div
      class={classes()}
      role="group"
      {...others}
    >
      {local.children}
    </div>
  );
};
