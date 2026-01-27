import { Component, JSX, Show, createSignal } from "solid-js";
import { ChevronRight } from "lucide-solid";
import "./accordion.css";

interface AccordionItemProps {
  value: string;
  title: string | JSX.Element;
  children: JSX.Element;
  defaultOpen?: boolean;
  icon?: JSX.Element;
  disabled?: boolean;
}

export const AccordionItem: Component<AccordionItemProps> = (props) => {
  // Simple internal state for standalone usage, but could be controlled if we implemented a Root
  const [isOpen, setIsOpen] = createSignal(props.defaultOpen ?? false);

  const toggle = () => {
    if (props.disabled) return;
    setIsOpen(!isOpen());
  };

  const id = `accordion-${Math.random().toString(36).substr(2, 9)}`;
  const contentId = `${id}-content`;

  return (
    <div class="accordion-item">
      <button
        type="button"
        class="accordion-header"
        onClick={toggle}
        aria-expanded={isOpen()}
        aria-controls={contentId}
        disabled={props.disabled}
      >
        <span class="accordion-trigger">
          <ChevronRight 
            size={16} 
            class="accordion-icon" 
          />
          {props.title}
        </span>
        <Show when={props.icon}>
          <div class="accordion-icon-wrapper">{props.icon}</div>
        </Show>
      </button>
      
      <div 
        id={contentId}
        class="accordion-content"
        data-state={isOpen() ? "open" : "closed"}
        role="region"
        aria-labelledby={id}
      >
        <div class="accordion-inner">
          {props.children}
        </div>
      </div>
    </div>
  );
};

interface AccordionProps {
  children: JSX.Element;
  class?: string;
}

export const Accordion: Component<AccordionProps> = (props) => {
    return (
        <div class={`accordion-root ${props.class || ""}`}>
            {props.children}
        </div>
    );
};
