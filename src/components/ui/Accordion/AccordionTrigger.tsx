import { Component, splitProps, JSX, Show } from 'solid-js';
import { ChevronRight } from 'lucide-solid';
import { cn } from '../../../lib/utils';
import { AccordionTriggerProps } from './types';
import { useAccordion, useAccordionItem } from './useAccordion';

/**
 * The interactive element that toggles the expansion of an AccordionItem.
 * Must be used as a child of AccordionItem.
 *
 * @param {AccordionTriggerProps} props - Configuration for the trigger button.
 * @returns {JSX.Element} An accessible button element.
 *
 * @example
 * ```tsx
 * <AccordionTrigger>
 *   <span>Section Title</span>
 * </AccordionTrigger>
 * ```
 */
export const AccordionTrigger: Component<AccordionTriggerProps> = props => {
    const [localProps, restProps] = splitProps(props, ['class', 'children', 'ref']);
    const accordionRootContext = useAccordion();
    const currentItemContext = useAccordionItem();

    /** Handles the click event to toggle this item's expansion state if not disabled. */
    const handleToggleInteraction = () => {
        if (!currentItemContext.disabled()) {
            accordionRootContext.toggleItem(currentItemContext.value());
        }
    };

    return (
        <button
            type="button"
            ref={localProps.ref}
            id={currentItemContext.triggerId}
            class={cn('ui-accordion-trigger', localProps.class)}
            aria-expanded={currentItemContext.isExpanded()}
            aria-controls={currentItemContext.contentId}
            aria-disabled={currentItemContext.disabled()}
            disabled={currentItemContext.disabled()}
            onClick={handleToggleInteraction}
            {...restProps}
        >
            {localProps.children}
        </button>
    );
};

/**
 * A pre-styled header component for frequent use cases.
 * Combines a toggle chevron, title text, and an optional icon into a cohesive UI element.
 *
 * @param {Object} props - Properties for the accordion header.
 * @param {string | JSX.Element} props.title - The main label for the section.
 * @param {JSX.Element} [props.icon] - An optional icon to display on the right.
 * @param {string} [props.class] - Optional CSS class for styling.
 * @returns {JSX.Element} A configured AccordionTrigger.
 *
 * @example
 * ```tsx
 * <AccordionHeader title="General Settings" icon={<SettingsIcon />} />
 * ```
 */
export const AccordionHeader: Component<{
    title: string | JSX.Element;
    icon?: JSX.Element;
    class?: string;
    ref?: (element: HTMLButtonElement) => void;
}> = props => {
    return (
        <AccordionTrigger class={props.class} ref={props.ref}>
            <span class="ui-accordion-trigger-content">
                <AccordionChevron />
                <span class="ui-accordion-title">{props.title}</span>
            </span>
            <Show when={props.icon}>
                <span class="ui-accordion-icon" aria-hidden="true">
                    {props.icon}
                </span>
            </Show>
        </AccordionTrigger>
    );
};

/**
 * The standard arrow icon used within the accordion headers.
 * It provides a visual cue for the expansion state and rotates automatically via CSS.
 *
 * @param {Object} props - Properties for the chevron icon.
 * @param {string} [props.class] - Optional CSS class for the icon.
 * @param {number} [props.size] - The size of the chevron in pixels. Defaults to 16.
 * @returns {JSX.Element} The rendered Lucide chevron icon.
 *
 * @example
 * ```tsx
 * <AccordionChevron size={20} class="custom-chevron" />
 * ```
 */
export const AccordionChevron: Component<{ class?: string; size?: number }> = props => {
    return <ChevronRight size={props.size ?? 16} class={cn('ui-accordion-chevron', props.class)} />;
};
