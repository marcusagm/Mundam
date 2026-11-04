import { Component, splitProps, createMemo, Show } from 'solid-js';
import { cn } from '../../../lib/utils';
import { createControllableSignal } from '../../../lib/primitives';
import { createId } from '../../../lib/primitives/createId';
import { SwitchProperties } from './types';
import './switch.css';

/**
 * Switch component for toggling between two states.
 * Follows WAI-ARIA switch pattern for accessibility and Mundam design standards.
 *
 * @param {SwitchProperties} properties - Properties for the Switch component.
 * @returns {JSX.Element} The rendered switch component.
 *
 * @example
 * <Switch label="Notifications" defaultChecked onCheckedChange={console.log} />
 *
 * @example
 * <Switch
 *   checked={isEnabled()}
 *   onCheckedChange={setIsEnabled}
 *   description="Enable or disable system alerts."
 *   size="lg"
 * />
 */
export const Switch: Component<SwitchProperties> = properties => {
    const [localProperties, remainingProperties] = splitProps(properties, [
        'class',
        'checked',
        'defaultChecked',
        'onCheckedChange',
        'label',
        'description',
        'size',
        'id',
        'disabled',
        'aria-label',
        'aria-labelledby'
    ]);

    const componentId = createMemo(() => localProperties.id || createId('switch'));
    const descriptionId = createMemo(() =>
        localProperties.description ? `${componentId()}-desc` : undefined
    );

    const { value: isChecked, setValue: setChecked } = createControllableSignal({
        value: () => localProperties.checked,
        defaultValue: localProperties.defaultChecked ?? false,
        onChange: (checked: boolean) => localProperties.onCheckedChange?.(checked)
    });

    /**
     * Toggles the checked state.
     */
    const handleSwitchClick = () => {
        if (localProperties.disabled) return;
        setChecked(!isChecked());
    };

    /**
     * Handles keyboard events for accessibility (Space and Enter).
     * @param {KeyboardEvent} event - The keyboard event.
     */
    const handleSwitchKeyDown = (event: KeyboardEvent) => {
        if (localProperties.disabled) return;
        if (event.key === ' ' || event.key === 'Enter') {
            event.preventDefault();
            setChecked(!isChecked());
        }
    };

    const activeSize = () => localProperties.size || 'md';

    return (
        <label
            class={cn(
                'ui-switch-wrapper',
                localProperties.disabled && 'ui-switch-disabled',
                localProperties.class
            )}
            for={componentId()}
        >
            <button
                type="button"
                role="switch"
                id={componentId()}
                class={cn(
                    'ui-switch',
                    `ui-switch-${activeSize()}`,
                    isChecked() && 'ui-switch-checked'
                )}
                aria-checked={isChecked()}
                aria-label={localProperties['aria-label'] || localProperties.label}
                aria-labelledby={localProperties['aria-labelledby']}
                aria-disabled={localProperties.disabled}
                aria-describedby={descriptionId()}
                disabled={localProperties.disabled}
                onClick={handleSwitchClick}
                onKeyDown={handleSwitchKeyDown}
            >
                <span class="ui-switch-thumb" />
            </button>

            <Show when={localProperties.label || localProperties.description}>
                <div class="ui-switch-content">
                    <Show when={localProperties.label}>
                        <span class={cn('ui-switch-label', `ui-switch-label-${activeSize()}`)}>
                            {localProperties.label}
                        </span>
                    </Show>
                    <Show when={localProperties.description}>
                        <span
                            id={descriptionId()}
                            class={cn(
                                'ui-switch-description',
                                `ui-switch-description-${activeSize()}`
                            )}
                        >
                            {localProperties.description}
                        </span>
                    </Show>
                </div>
            </Show>

            {/* Hidden input for form submission compatibility */}
            <input
                type="checkbox"
                checked={isChecked()}
                disabled={localProperties.disabled}
                class="ui-switch-input"
                tabindex={-1}
                aria-hidden="true"
                {...remainingProperties}
            />
        </label>
    );
};
