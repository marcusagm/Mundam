import { Component, JSX, splitProps, Show, onCleanup } from 'solid-js';
import { cn } from '../../lib/utils';
import { useInput, SCOPE_PRIORITIES } from '../../core/input';
import './input.css';

export type InputSize = 'sm' | 'md' | 'lg';

export interface InputProps extends JSX.InputHTMLAttributes<HTMLInputElement> {
    /** Label to display above the input */
    label?: string;
    /** Icon to display on the left side */
    leftIcon?: JSX.Element;
    /** Icon to display on the right side */
    rightIcon?: JSX.Element;
    /** Size variant */
    size?: InputSize;
    /** Error state */
    error?: boolean;
    /** Error message to display */
    errorMessage?: string;
    /** Wrapper class */
    wrapperClass?: string;
}

/**
 * Input component with support for icons, sizes, and error states.
 *
 * @example
 * // Basic usage
 * <Input placeholder="Enter your name" />
 *
 * @example
 * // With icon
 * <Input leftIcon={<Search size={16} />} placeholder="Search..." />
 *
 * @example
 * // With error
 * <Input error errorMessage="This field is required" />
 */
export const Input: Component<InputProps> = props => {
    const [local, others] = splitProps(props, [
        'label',
        'leftIcon',
        'rightIcon',
        'size',
        'error',
        'errorMessage',
        'wrapperClass',
        'class'
    ]);

    const input = useInput();
    let isEditingScopeActive = false;

    const size = () => local.size || 'md';

    const handleFocus = (e: FocusEvent) => {
        if (!isEditingScopeActive) {
            input.pushScope('editing', SCOPE_PRIORITIES.editing, true);
            isEditingScopeActive = true;
        }
        if (typeof others.onFocus === 'function') {
            (others.onFocus as any)(e);
        }
    };

    const handleBlur = (e: FocusEvent) => {
        if (isEditingScopeActive) {
            input.popScope('editing');
            isEditingScopeActive = false;
        }
        if (typeof others.onBlur === 'function') {
            (others.onBlur as any)(e);
        }
    };

    const handleKeyDown = (e: KeyboardEvent) => {
        // Stop propagation for keys that are common shortcuts
        // to prevent them from bubbling up to the global shortcut system
        // while the user is typing in an input.
        if (
            [
                'Enter',
                'ArrowUp',
                'ArrowDown',
                'ArrowLeft',
                'ArrowRight',
                ' ',
                'Home',
                'End'
            ].includes(e.key)
        ) {
            e.stopPropagation();
            if (e.key === 'Enter') {
                e.preventDefault();
            }
        }
        if (typeof others.onKeyDown === 'function') {
            (others.onKeyDown as any)(e);
        }
    };

    onCleanup(() => {
        if (isEditingScopeActive) {
            input.popScope('editing');
        }
    });

    return (
        <div class={cn('ui-input-wrapper', local.wrapperClass)}>
            <Show when={local.label}>
                <label class="ui-input-label">{local.label}</label>
            </Show>
            <div
                class={cn(
                    'ui-input-container',
                    `ui-input-${size()}`,
                    local.error && 'ui-input-error',
                    others.disabled && 'ui-input-disabled',
                    !!local.leftIcon && 'ui-input-has-left',
                    !!local.rightIcon && 'ui-input-has-right'
                )}
            >
                <Show when={local.leftIcon}>
                    <span class="ui-input-icon ui-input-icon-left">{local.leftIcon}</span>
                </Show>

                <input
                    class={cn('ui-input', local.class)}
                    aria-invalid={local.error || undefined}
                    aria-describedby={
                        local.error && local.errorMessage ? `${others.id}-error` : undefined
                    }
                    {...others}
                    onFocus={handleFocus}
                    onBlur={handleBlur}
                    onKeyDown={handleKeyDown}
                />

                <Show when={local.rightIcon}>
                    <span class="ui-input-icon ui-input-icon-right">{local.rightIcon}</span>
                </Show>
            </div>

            <Show when={local.error && local.errorMessage}>
                <span id={`${others.id}-error`} class="ui-input-error-message" role="alert">
                    {local.errorMessage}
                </span>
            </Show>
        </div>
    );
};
