import { Component, splitProps, For, Show } from 'solid-js';
import { ChevronDown as ChevronDownIcon, X as CloseIcon } from 'lucide-solid';
import { cn as concatenateClasses } from '../../../lib/utils';
import { Root as SelectRoot } from './Root';
import { Trigger as SelectTrigger } from './Trigger';
import { Content as SelectContent } from './Content';
import { Item as SelectItem } from './Item';
import { Value as SelectValue } from './Value';
import { Search as SelectSearch } from './Search';
import { SelectProperties, SelectOption } from './types';
import './select.css';

/**
 * Select component for choosing from a list of options.
 * A high-level components that simplifies usage while maintaining flexibility.
 *
 * @param {SelectProperties} properties - Properties for the Select component.
 * @returns {JSX.Element} The rendered select field.
 *
 * @example
 * <Select options={[{ value: '1', label: 'One' }, { value: '2', label: 'Two' }]} placeholder="Pick..." />
 */
export const Select: Component<SelectProperties> = properties => {
    const [localProperties, remainingProperties] = splitProps(properties, [
        'class',
        'options',
        'value',
        'defaultValue',
        'onValueChange',
        'placeholder',
        'disabled',
        'clearable',
        'searchable',
        'name',
        'error',
        'errorMessage',
        'id',
        'leftIcon',
        'rightIcon',
        'size',
        'aria-label',
        'aria-labelledby',
        'aria-describedby'
    ]);

    /**
     * Filters options based on search query for convenience.
     */
    const filteredOptions = (searchQuery: string): SelectOption[] => {
        if (!localProperties.searchable || !searchQuery) {
            return localProperties.options;
        }
        const queryTerm = searchQuery.toLowerCase();
        return localProperties.options.filter(option =>
            option.label.toLowerCase().includes(queryTerm)
        );
    };

    const activeSize = () => localProperties.size || 'md';

    return (
        <SelectRoot
            value={localProperties.value}
            defaultValue={localProperties.defaultValue}
            onValueChange={localProperties.onValueChange}
            disabled={localProperties.disabled}
            name={localProperties.name}
            options={localProperties.options}
            id={localProperties.id}
        >
            {context => (
                <div
                    class={concatenateClasses('ui-select', localProperties.class)}
                    {...remainingProperties}
                >
                    <SelectTrigger
                        aria-label={localProperties['aria-label']}
                        aria-labelledby={localProperties['aria-labelledby']}
                        aria-describedby={localProperties['aria-describedby']}
                        error={localProperties.error}
                        size={activeSize()}
                        class={concatenateClasses(
                            !!localProperties.leftIcon && 'ui-select-has-left-icon',
                            !!localProperties.rightIcon && 'ui-select-has-right-icon'
                        )}
                    >
                        <Show when={localProperties.leftIcon}>
                            <span class="ui-select-icon-left">{localProperties.leftIcon}</span>
                        </Show>

                        <SelectValue placeholder={localProperties.placeholder} />

                        <Show when={localProperties.rightIcon}>
                            <span class="ui-select-icon-right">{localProperties.rightIcon}</span>
                        </Show>

                        <div class="ui-select-icons">
                            <Show when={localProperties.clearable && context.value()}>
                                <span
                                    class="ui-select-clear"
                                    onClick={event => {
                                        event.stopPropagation();
                                        context.setValue('');
                                    }}
                                    aria-label="Clear selected value"
                                >
                                    <CloseIcon size={14} />
                                </span>
                            </Show>
                            <ChevronDownIcon
                                size={16}
                                class={concatenateClasses(
                                    'ui-select-chevron',
                                    context.isOpen() && 'ui-select-chevron-open'
                                )}
                            />
                        </div>
                    </SelectTrigger>

                    <SelectContent>
                        <Show when={localProperties.searchable}>
                            <SelectSearch />
                        </Show>

                        <div class="ui-select-options">
                            <Show
                                when={filteredOptions(context.searchQuery()).length > 0}
                                fallback={<div class="ui-select-empty">No options found</div>}
                            >
                                <For each={filteredOptions(context.searchQuery())}>
                                    {option => <SelectItem option={option} />}
                                </For>
                            </Show>
                        </div>
                    </SelectContent>

                    <Show when={localProperties.error && localProperties.errorMessage}>
                        <span class="ui-select-error-message" role="alert">
                            {localProperties.errorMessage}
                        </span>
                    </Show>

                    <input type="hidden" name={localProperties.name} value={context.value()} />
                </div>
            )}
        </SelectRoot>
    );
};
