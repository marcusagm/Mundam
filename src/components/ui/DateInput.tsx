import { Component, createEffect, createSignal, splitProps, mergeProps } from 'solid-js';

import { DatePicker } from './DatePicker';
import { Popover } from './Popover';
import { Calendar } from 'lucide-solid';
import { cn } from '../../lib/utils';
import './date-input.css';

import { InputProps } from './Input';

export interface DateInputProps extends Omit<
    InputProps,
    'value' | 'onChange' | 'onInput' | 'defaultValue'
> {
    value?: Date | null;
    defaultValue?: Date | null;
    onChange?: (date: Date | null) => void;
    /** Format of date: Currently hardcoded DD/MM/YYYY in mask, but prop could allow formats later */
    placeholder?: string;
}

// Helper to format Date to DD/MM/YYYY
const formatDate = (date: Date): string => {
    const d = date.getDate().toString().padStart(2, '0');
    const m = (date.getMonth() + 1).toString().padStart(2, '0');
    const y = date.getFullYear().toString();
    return `${d}/${m}/${y}`;
};

// Helper to parse DD/MM/YYYY to Date
const parseDate = (str: string): Date | null => {
    if (!str || str.length < 10) return null;
    const parts = str.split('/');
    if (parts.length !== 3) return null;
    const d = parseInt(parts[0], 10);
    const m = parseInt(parts[1], 10) - 1;
    const y = parseInt(parts[2], 10);

    const date = new Date(y, m, d);
    if (date.getFullYear() === y && date.getMonth() === m && date.getDate() === d) {
        return date;
    }
    return null;
};

export const DateInput: Component<DateInputProps> = props => {
    const merged = mergeProps({}, props);
    const [local, others] = splitProps(merged, [
        'value',
        'defaultValue',
        'onChange',
        'class',
        'wrapperClass',
        'label',
        'error',
        'errorMessage',
        'disabled',
        'size'
    ]);

    // Internal string state for the input
    const [inputValue, setInputValue] = createSignal('');
    const [isPopoverOpen, setIsPopoverOpen] = createSignal(false);

    // Sync prop value to input string
    createEffect(() => {
        const v = local.value ?? local.defaultValue;
        if (v) {
            setInputValue(formatDate(v));
        } else {
            // Only reset if empty to avoid fighting user input?
            // If value is controlled and becomes null, we should clear it.
            // If undefined, might be uncontrolled initial.
            if (local.value === null) setInputValue('');
        }
    });

    const applyMask = (val: string) => {
        const mask = '99/99/9999';
        let result = '';
        let valIndex = 0;
        const cleanVal = val.replace(/\D/g, '');

        for (let i = 0; i < mask.length && valIndex < cleanVal.length; i++) {
            const maskChar = mask[i];
            if (maskChar === '9') {
                result += cleanVal[valIndex];
                valIndex++;
            } else {
                result += maskChar;
            }
        }
        return result;
    };

    const handleInput = (e: InputEvent & { currentTarget: HTMLInputElement }) => {
        const raw = e.currentTarget.value;
        const masked = applyMask(raw);

        // Update input execution
        // Note: manipulating value directly on input for mask effect
        if (masked !== raw) {
            e.currentTarget.value = masked;
        }
        setInputValue(masked);

        const date = parseDate(masked);
        if (date) {
            local.onChange?.(date);
        } else if (masked === '') {
            local.onChange?.(null);
        }
    };

    const handleDateSelect = (date: Date) => {
        setInputValue(formatDate(date));
        local.onChange?.(date);
        setIsPopoverOpen(false);
    };

    const triggerButton = (
        <button
            type="button"
            class="ui-date-input-trigger"
            onClick={e => {
                e.stopPropagation();
                if (!local.disabled) setIsPopoverOpen(!isPopoverOpen());
            }}
            disabled={local.disabled}
            aria-label="Open date picker"
            tabIndex={-1}
        >
            <Calendar size={16} />
        </button>
    );

    return (
        <div class={cn('ui-date-input-wrapper', local.wrapperClass)}>
            {local.label && <label class="ui-date-input-label">{local.label}</label>}

            <div
                class={cn(
                    'ui-date-input-container',
                    `ui-date-input-${local.size || 'md'}`,
                    local.error && 'ui-date-input-error',
                    local.disabled && 'ui-date-input-disabled',
                    'ui-date-input-has-right', // Always has calendar icon
                    local.class
                )}
            >
                <input
                    type="text"
                    class="ui-date-input-field"
                    value={inputValue()}
                    onInput={handleInput}
                    disabled={local.disabled}
                    placeholder={props.placeholder || 'DD/MM/YYYY'}
                    aria-invalid={local.error || undefined}
                    {...others}
                />

                <div class="ui-date-input-icon-right">
                    {local.disabled ? (
                        triggerButton
                    ) : (
                        <Popover
                            isOpen={isPopoverOpen()}
                            onClose={() => setIsPopoverOpen(false)}
                            trigger={triggerButton}
                            class="ui-date-input-popover"
                            align="end"
                        >
                            <DatePicker
                                value={parseDate(inputValue()) || (local.value ?? new Date())}
                                onChange={handleDateSelect}
                            />
                        </Popover>
                    )}
                </div>
            </div>

            {local.error && local.errorMessage && (
                <span class="ui-date-input-error-message" role="alert">
                    {local.errorMessage}
                </span>
            )}
        </div>
    );
};
