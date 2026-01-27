import { 
  Component, 
  JSX, 
  splitProps, 
  Show, 
  createSignal, 
  createContext,
  useContext,
  Accessor,
} from "solid-js";
import { ChevronRight } from "lucide-solid";
import { cn } from "../../lib/utils";
import { createControllableSignal } from "../../lib/primitives";
import { createId } from "../../lib/primitives/createId";
import "./accordion.css";

// Context for Accordion
interface AccordionContextValue {
  type: "single" | "multiple";
  expandedItems: Accessor<string[]>;
  toggleItem: (value: string) => void;
}

const AccordionContext = createContext<AccordionContextValue>();

const useAccordion = () => useContext(AccordionContext);

// Accordion Root
export interface AccordionProps {
  /** Type of accordion: single or multiple items can be expanded */
  type?: "single" | "multiple";
  /** Controlled expanded items */
  value?: string[];
  /** Default expanded items (for uncontrolled) */
  defaultValue?: string[];
  /** Callback when expanded items change */
  onValueChange?: (value: string[]) => void;
  /** Whether all items should be collapsible (for single type) */
  collapsible?: boolean;
  /** Additional CSS class */
  class?: string;
  children: JSX.Element;
}

/**
 * Accordion component for expandable content sections.
 * 
 * @example
 * // Single (only one item open at a time)
 * <Accordion type="single" defaultValue={["item-1"]}>
 *   <AccordionItem value="item-1" title="Section 1">
 *     Content for section 1
 *   </AccordionItem>
 *   <AccordionItem value="item-2" title="Section 2">
 *     Content for section 2
 *   </AccordionItem>
 * </Accordion>
 * 
 * // Multiple (multiple items can be open)
 * <Accordion type="multiple">
 *   <AccordionItem value="item-1" title="Section 1">...</AccordionItem>
 *   <AccordionItem value="item-2" title="Section 2">...</AccordionItem>
 * </Accordion>
 */
export const Accordion: Component<AccordionProps> = (props) => {
  const [local] = splitProps(props, [
    "type",
    "value",
    "defaultValue",
    "onValueChange",
    "collapsible",
    "class",
    "children",
  ]);

  const type = () => local.type || "single";
  const collapsible = () => local.collapsible ?? true;

  const { value: expandedItems, setValue: setExpandedItems } = createControllableSignal<string[]>({
    value: () => local.value,
    defaultValue: local.defaultValue ?? [],
    onChange: local.onValueChange,
  });

  const toggleItem = (itemValue: string) => {
    const current = expandedItems();
    const isExpanded = current.includes(itemValue);

    if (type() === "single") {
      if (isExpanded && collapsible()) {
        setExpandedItems([]);
      } else if (!isExpanded) {
        setExpandedItems([itemValue]);
      }
    } else {
      // Multiple type
      if (isExpanded) {
        setExpandedItems(current.filter((v: string) => v !== itemValue));
      } else {
        setExpandedItems([...current, itemValue]);
      }
    }
  };

  const contextValue: AccordionContextValue = {
    type: type(),
    expandedItems,
    toggleItem,
  };

  return (
    <AccordionContext.Provider value={contextValue}>
      <div
        class={cn("ui-accordion", local.class)}
        data-orientation="vertical"
      >
        {local.children}
      </div>
    </AccordionContext.Provider>
  );
};

// Accordion Item
export interface AccordionItemProps {
  /** Unique identifier for this item */
  value: string;
  /** Title/trigger content */
  title: string | JSX.Element;
  /** Collapsible content */
  children: JSX.Element;
  /** Optional icon displayed on the right */
  icon?: JSX.Element;
  /** Whether this item is disabled */
  disabled?: boolean;
  /** Whether this item is open by default (only works outside of Accordion context) */
  defaultOpen?: boolean;
  /** Additional CSS class */
  class?: string;
}

export const AccordionItem: Component<AccordionItemProps> = (props) => {
  const [local] = splitProps(props, [
    "value",
    "title",
    "children",
    "icon",
    "disabled",
    "defaultOpen",
    "class",
  ]);

  const context = useAccordion();
  
  // Fallback for standalone usage without Accordion wrapper
  const [standaloneOpen, setStandaloneOpen] = createSignal(local.defaultOpen ?? false);

  const isOpen = () => {
    if (context) {
      return context.expandedItems().includes(local.value);
    }
    return standaloneOpen();
  };

  const toggle = () => {
    if (local.disabled) return;
    
    if (context) {
      context.toggleItem(local.value);
    } else {
      setStandaloneOpen(!standaloneOpen());
    }
  };

  const triggerId = createId("accordion-trigger");
  const contentId = createId("accordion-content");

  return (
    <div
      class={cn(
        "ui-accordion-item",
        isOpen() && "ui-accordion-item-open",
        local.disabled && "ui-accordion-item-disabled",
        local.class
      )}
      data-state={isOpen() ? "open" : "closed"}
    >
      <button
        type="button"
        id={triggerId}
        class="ui-accordion-trigger"
        aria-expanded={isOpen()}
        aria-controls={contentId}
        aria-disabled={local.disabled}
        disabled={local.disabled}
        onClick={toggle}
      >
        <span class="ui-accordion-trigger-content">
          <ChevronRight
            size={16}
            class="ui-accordion-chevron"
          />
          <span class="ui-accordion-title">{local.title}</span>
        </span>
        <Show when={local.icon}>
          <span class="ui-accordion-icon" aria-hidden="true">
            {local.icon}
          </span>
        </Show>
      </button>

      <div
        id={contentId}
        role="region"
        aria-labelledby={triggerId}
        class="ui-accordion-content"
        data-state={isOpen() ? "open" : "closed"}
        hidden={!isOpen()}
      >
        <div class="ui-accordion-content-inner">
          {local.children}
        </div>
      </div>
    </div>
  );
};
