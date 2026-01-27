import { Component, JSX, splitProps } from "solid-js";
import { cn } from "../../lib/utils";
import "./kbd.css";

export interface KbdProps extends JSX.HTMLAttributes<HTMLElement> {
  children: JSX.Element;
}

/**
 * Keyboard key indicator component.
 * Displays keyboard shortcuts in a visually distinct style.
 * 
 * @example
 * <Kbd>⌘</Kbd>
 * <Kbd>Ctrl</Kbd> + <Kbd>C</Kbd>
 */
export const Kbd: Component<KbdProps> = (props) => {
  const [local, others] = splitProps(props, ["class", "children"]);

  return (
    <kbd class={cn("ui-kbd", local.class)} {...others}>
      {local.children}
    </kbd>
  );
};
