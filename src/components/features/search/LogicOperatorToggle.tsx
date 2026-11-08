import { Component, Show } from 'solid-js';
import { cn } from '../../../lib/utils';
import { type LogicalOperator } from '../../../core/store/filter';

/**
 * Properties for the LogicOperatorToggle component.
 */
interface LogicOperatorToggleProperties {
    /** The active logical operator ('and' | 'or'). */
    logicalOperator: LogicalOperator;
    /** Callback invoked when the user selects a new operator. */
    onOperatorChange: (selectedOperator: LogicalOperator) => void;
    /** Whether to render compact button labels for deeply nested groups. */
    isCompact?: boolean;
    /** Additional CSS class names. */
    class?: string;
}

/**
 * Segmented pill control for switching between 'AND' (all) and 'OR' (any) boolean operators.
 *
 * @param properties - Properties for the component.
 * @returns The rendered segmented switch.
 */
export const LogicOperatorToggle: Component<LogicOperatorToggleProperties> = properties => {
    return (
        <div
            class={cn('logic-operator-toggle', properties.class)}
            role="radiogroup"
            aria-label="Condition matching rule"
        >
            <button
                type="button"
                role="radio"
                aria-checked={properties.logicalOperator === 'and'}
                class={cn(
                    'logic-operator-button',
                    properties.logicalOperator === 'and' && 'is-active'
                )}
                onClick={() => properties.onOperatorChange('and')}
            >
                <Show when={properties.isCompact} fallback="All conditions (AND)">
                    All (AND)
                </Show>
            </button>
            <button
                type="button"
                role="radio"
                aria-checked={properties.logicalOperator === 'or'}
                class={cn(
                    'logic-operator-button',
                    properties.logicalOperator === 'or' && 'is-active'
                )}
                onClick={() => properties.onOperatorChange('or')}
            >
                <Show when={properties.isCompact} fallback="Any condition (OR)">
                    Any (OR)
                </Show>
            </button>
        </div>
    );
};
