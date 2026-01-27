import { Accessor, createSignal, createMemo } from "solid-js";

export interface ControllableSignalOptions<T> {
  /** The controlled value (external) */
  value?: Accessor<T | undefined>;
  /** Default value if uncontrolled */
  defaultValue?: T;
  /** Callback when value changes (required for controlled mode) */
  onChange?: (value: T) => void;
}

export interface ControllableSignalReturn<T> {
  /** Current value (reactive) */
  value: Accessor<T>;
  /** Set value (handles both controlled and uncontrolled) */
  setValue: (next: T) => void;
  /** Whether component is in controlled mode */
  isControlled: Accessor<boolean>;
}

/**
 * Creates a signal that can be either controlled or uncontrolled.
 * Follows the pattern used by Radix and Shadcn for form components.
 * 
 * @example
 * // Uncontrolled (internal state)
 * const { value, setValue } = createControllableSignal({ defaultValue: false });
 * 
 * // Controlled (external state)
 * const [checked, setChecked] = createSignal(false);
 * const { value, setValue } = createControllableSignal({ 
 *   value: checked, 
 *   onChange: setChecked 
 * });
 */
export function createControllableSignal<T>(
  options: ControllableSignalOptions<T>
): ControllableSignalReturn<T> {
  const isControlled = createMemo(() => options.value?.() !== undefined);
  
  // Internal state for uncontrolled mode
  const [internalValue, setInternalValue] = createSignal<T>(
    options.defaultValue as T
  );

  const value = createMemo(() => {
    if (isControlled()) {
      return options.value!() as T;
    }
    return internalValue();
  });

  const setValue = (next: T) => {
    if (isControlled()) {
      options.onChange?.(next);
    } else {
      setInternalValue(() => next);
      options.onChange?.(next);
    }
  };

  return { value, setValue, isControlled };
}
