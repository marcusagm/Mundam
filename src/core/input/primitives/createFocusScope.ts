/**
 * createFocusScope Primitive
 * Activates an input scope only while keyboard focus is inside a container.
 */

import { createSignal, type Accessor } from 'solid-js';
import { createConditionalScope } from './createInputScope';
import type { InputScopeName } from '../types';

interface FocusScopeHandle {
    /** Whether focus is currently inside the container */
    isFocused: Accessor<boolean>;
    /** Bind to the container's `onFocusIn` */
    onFocusIn: () => void;
    /** Bind to the container's `onFocusOut` */
    onFocusOut: (event: FocusEvent & { currentTarget: HTMLElement }) => void;
    /** Wraps a command handler so it only runs for the focused container */
    whenFocused: <Arguments extends unknown[]>(
        handler: (...handlerArguments: Arguments) => void
    ) => (...handlerArguments: Arguments) => void;
}

/**
 * Pushes `name` to the scope stack while focus is within the container that receives
 * the returned handlers. Commands are broadcast to every subscriber, so wrap handlers
 * with `whenFocused` when several instances of the component can be mounted at once.
 *
 * @example
 * const focusScope = createFocusScope('video-player');
 * useCommands({ 'video:toggle-play': focusScope.whenFocused(togglePlay) });
 * <div onFocusIn={focusScope.onFocusIn} onFocusOut={focusScope.onFocusOut} />
 */
export function createFocusScope(name: InputScopeName, priority?: number): FocusScopeHandle {
    const [isFocused, setIsFocused] = createSignal(false);

    createConditionalScope(name, isFocused, priority);

    return {
        isFocused,
        onFocusIn: () => setIsFocused(true),
        onFocusOut: event => {
            const nextTarget = event.relatedTarget;
            if (nextTarget instanceof Node && event.currentTarget.contains(nextTarget)) return;
            setIsFocused(false);
        },
        whenFocused:
            handler =>
            (...handlerArguments) => {
                if (isFocused()) handler(...handlerArguments);
            }
    };
}
