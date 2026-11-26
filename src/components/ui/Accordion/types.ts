import { JSX, Accessor } from 'solid-js';

/**
 * Defines the expansion behavior of the accordion.
 * - 'single': Permits only one item to be expanded at unknown given time.
 * - 'multiple': Allows multiple items to be expanded simultaneously.
 */
export type AccordionType = 'single' | 'multiple';

/**
 * Properties for the root Accordion component.
 */
export interface AccordionRootProps {
    /** Whether the accordion supports single or multiple expanded items. Defaults to 'single'. */
    type?: AccordionType;
    /** Controlled array of expanded item values. */
    value?: string[];
    /** Initial array of expanded item values (uncontrolled mode). Defaults to an empty array. */
    defaultValue?: string[];
    /** Callback function triggered when the set of expanded items changes. */
    onValueChange?: (valueList: string[]) => void;
    /** For 'single' mode: whether the active item can be collapsed by clicking it again. Defaults to true. */
    collapsible?: boolean;
    /** Whether the entire accordion interaction is disabled. */
    disabled?: boolean;
    /** Optional CSS class to apply to the root accordion element. */
    class?: string;
    /** Children elements, typically a collection of AccordionItem components. */
    children?: JSX.Element;
}

/**
 * Properties for individual Accordion items.
 */
export interface AccordionItemProps {
    /** A unique identifier for this specific item within the accordion. */
    value: string;
    /** Whether this specific item is disabled and cannot be toggled. */
    disabled?: boolean;
    /** Optional CSS class for the item container element. */
    class?: string;
    /** Children elements, typically AccordionTrigger and AccordionContent. */
    children?: JSX.Element;
}

/**
 * Properties for the Accordion trigger element (the interactive header).
 */
export interface AccordionTriggerProps {
    /** Ref to the focusable trigger. */
    ref?: (element: HTMLButtonElement) => void;
    /** Optional CSS class for the trigger button. */
    class?: string;
    /** Children elements to render inside the trigger button (e.g., text, icons). */
    children?: JSX.Element;
}

/**
 * Properties for the Accordion content section (the expandable panel).
 */
export interface AccordionContentProps {
    /** Optional CSS class for the content container. */
    class?: string;
    /** Children elements to render inside the expanded content section. */
    children?: JSX.Element;
}

/**
 * Internal context value shared from the Accordion root to its sub-components.
 */
export interface AccordionContextValue {
    /** A reactive accessor for the array of currently expanded item identifiers. */
    expandedItems: Accessor<string[]>;
    /** Function to toggle the expansion state of a specific item identifier. */
    toggleItem: (itemIdentifier: string) => void;
    /** A reactive accessor indicating if the entire accordion is currently disabled. */
    disabled: Accessor<boolean>;
    /** A reactive accessor for the current accordion behavior type ('single' or 'multiple'). */
    type: Accessor<AccordionType>;
}

/**
 * Internal context value shared within an individual AccordionItem.
 */
export interface AccordionItemContextValue {
    /** A reactive accessor for the unique identifier of the current item. */
    value: Accessor<string>;
    /** A reactive accessor indicating if this specific item is disabled. */
    disabled: Accessor<boolean>;
    /** A reactive accessor indicating if this specific item is currently expanded. */
    isExpanded: Accessor<boolean>;
    /** A unique identifier for the trigger element, used for ARIA accessibility relationships. */
    triggerId: string;
    /** A unique identifier for the content element, used for ARIA accessibility relationships. */
    contentId: string;
}
