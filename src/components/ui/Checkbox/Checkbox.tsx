import { Component, splitProps, createMemo, Show } from 'solid-js';
import { cn as concatenateClasses } from '../../../lib/utils';
import { createControllableSignal } from '../../../lib/primitives';
import { createId as generateUniqueId } from '../../../lib/primitives/createId';
import { Check as CheckIcon, Minus as IndeterminateIcon } from 'lucide-solid';
import { CheckboxProperties } from './types';
import './checkbox.css';

/**
 * Checkbox component for selecting multiple options.
 * Supports indeterminate state for partial selections.
 *
 * @param {CheckboxProperties} properties - Properties for the checkbox.
 * @returns {JSX.Element} A stylized checkbox button.
 *
 * @example
 * <Checkbox label="Accept terms" />
 *
 * @example
 * <Checkbox checked indeterminate label="Select all" />
 */
export const Checkbox: Component<CheckboxProperties> = properties => {
    const [localProperties, remainingProperties] = splitProps(properties, [
        'class',
        'checked',
        'defaultChecked',
        'indeterminate',
        'onCheckedChange',
        'label',
        'description',
        'size',
        'id',
        'disabled'
    ]);

    /**
     * Generates or uses a unique identifier for the checkbox.
     */
    const accessibleId = createMemo(() => localProperties.id || generateUniqueId('checkbox'));

    /**
     * Generates a unique identifier for the description to link it via aria-describedby.
     */
    const descriptionIdentifier = createMemo(() =>
        localProperties.description ? `${accessibleId()}-description` : undefined
    );

    /**
     * Internal state management for the checkbox toggle.
     */
    const { value: isChecked, setValue: setChecked } = createControllableSignal({
        value: () => localProperties.checked,
        defaultValue: localProperties.defaultChecked ?? false,
        onChange: (checked: boolean) => localProperties.onCheckedChange?.(checked)
    });

    /**
     * Handles user interaction to toggle the checkbox state.
     */
    const handleActionTrigger = (event: Event) => {
        event.preventDefault();
        event.stopPropagation();
        if (localProperties.disabled) {
            return;
        }
        setChecked(!isChecked());
    };

    /**
     * Accessibility support for keyboard interaction (Space/Enter).
     */
    const handleKeyboardInput = (event: KeyboardEvent) => {
        if (localProperties.disabled) {
            return;
        }
        if (event.key === ' ' || event.key === 'Enter') {
            event.preventDefault();
            setChecked(!isChecked());
        }
    };

    const activeSize = () => localProperties.size || 'md';

    return (
        <label
            class={concatenateClasses(
                'ui-checkbox-wrapper',
                localProperties.disabled && 'ui-checkbox-disabled',
                localProperties.class
            )}
            for={accessibleId()}
        >
            <button
                type="button"
                role="checkbox"
                id={accessibleId()}
                class={concatenateClasses(
                    'ui-checkbox',
                    `ui-checkbox-${activeSize()}`,
                    (isChecked() || localProperties.indeterminate) && 'ui-checkbox-checked'
                )}
                aria-checked={localProperties.indeterminate ? 'mixed' : isChecked()}
                aria-disabled={localProperties.disabled}
                aria-describedby={descriptionIdentifier()}
                disabled={localProperties.disabled}
                onClick={handleActionTrigger}
                onKeyDown={handleKeyboardInput}
            >
                <Show when={isChecked() && !localProperties.indeterminate}>
                    <CheckIcon size={12} class="ui-checkbox-icon" />
                </Show>
                <Show when={localProperties.indeterminate}>
                    <IndeterminateIcon size={12} class="ui-checkbox-icon" />
                </Show>
            </button>

            <Show when={localProperties.label || localProperties.description}>
                <div class="ui-checkbox-content">
                    <Show when={localProperties.label}>
                        <span
                            class={concatenateClasses(
                                'ui-checkbox-label',
                                `ui-checkbox-label-${activeSize()}`
                            )}
                        >
                            {localProperties.label}
                        </span>
                    </Show>
                    <Show when={localProperties.description}>
                        <span
                            id={descriptionIdentifier()}
                            class={concatenateClasses(
                                'ui-checkbox-description',
                                `ui-checkbox-description-${activeSize()}`
                            )}
                        >
                            {localProperties.description}
                        </span>
                    </Show>
                </div>
            </Show>

            <input
                type="checkbox"
                checked={isChecked()}
                disabled={localProperties.disabled}
                class="ui-checkbox-input"
                tabindex={-1}
                aria-hidden="true"
                {...remainingProperties}
            />
        </label>
    );
};
