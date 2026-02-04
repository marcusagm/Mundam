import { Component, JSX, Show, createSignal, onCleanup, createEffect } from "solid-js";
import { Portal } from "solid-js/web";
import { cn } from "../../lib/utils";
import "./popover.css";

export interface PopoverProps {
  trigger: JSX.Element;
  children: JSX.Element;
  class?: string;
  isOpen?: boolean;
  onClose?: () => void;
}

export const Popover: Component<PopoverProps> = (props) => {
  const [internalOpen, setInternalOpen] = createSignal(false);
  let triggerRef: HTMLDivElement | undefined;
  let contentRef: HTMLDivElement | undefined;

  const isOpen = () => props.isOpen ?? internalOpen();

  const toggle = (e: MouseEvent) => {
    e.stopPropagation();
    if (props.isOpen !== undefined) {
      if (isOpen()) props.onClose?.();
    } else {
      setInternalOpen(!internalOpen());
    }
  };

  const close = () => {
    if (props.isOpen !== undefined) {
      props.onClose?.();
    } else {
      setInternalOpen(false);
    }
  };

  createEffect(() => {
    if (!isOpen()) return;

    const handleClickOutside = (e: MouseEvent) => {
      if (!triggerRef?.contains(e.target as Node) && !contentRef?.contains(e.target as Node)) {
        close();
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    onCleanup(() => document.removeEventListener("mousedown", handleClickOutside));
  });

  const positionContent = () => {
    if (!triggerRef || !contentRef) return;
    const rect = triggerRef.getBoundingClientRect();
    const contentRect = contentRef.getBoundingClientRect();
    
    let top = rect.bottom + 8;
    let left = rect.right - contentRect.width;

    // Check bounds
    if (left < 10) left = 10;
    if (top + contentRect.height > window.innerHeight) {
        top = rect.top - contentRect.height - 8;
    }

    contentRef.style.top = `${top}px`;
    contentRef.style.left = `${left}px`;
  };

  createEffect(() => {
    if (isOpen()) {
        requestAnimationFrame(positionContent);
    }
  });

  return (
    <div 
      ref={triggerRef} 
      onClick={toggle} 
      class="ui-popover-wrapper" 
      style={{ display: "inline-block" }}
    >
      {props.trigger}
      <Show when={isOpen()}>
        <Portal>
          <div 
            ref={contentRef} 
            class={cn("ui-popover-content", props.class)}
            style={{ position: "fixed", "z-index": 10000 }}
          >
            {props.children}
          </div>
        </Portal>
      </Show>
    </div>
  );
};
