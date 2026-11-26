/**
 * Command Event System
 *
 * Pub/Sub pattern for dispatching and subscribing to named commands.
 * Commands are emitted when keyboard shortcuts fire with a `command` property.
 */

import type { ShortcutPayload } from './types';

type CommandHandler = (payload: ShortcutPayload) => void;
const commandHandlers = new Map<string, Set<CommandHandler>>();

/**
 * Subscribe to a command
 */
export function onCommand(command: string, handler: CommandHandler): () => void {
    if (!commandHandlers.has(command)) {
        commandHandlers.set(command, new Set());
    }

    commandHandlers.get(command)!.add(handler);

    // Return unsubscribe function
    return () => {
        const handlers = commandHandlers.get(command);
        if (handlers?.delete(handler) && handlers.size === 0) commandHandlers.delete(command);
    };
}

/**
 * Emit a command event
 */
export function emitCommand(command: string, payload: ShortcutPayload): void {
    const handlers = commandHandlers.get(command);
    if (!handlers) return;

    for (const handler of handlers) {
        try {
            handler(payload);
        } catch (error) {
            console.error(`[InputDispatcher] Error in command handler for ${command}:`, error);
        }
    }
}

/**
 * Clear all command handlers (for cleanup)
 */
export function clearCommandHandlers(): void {
    commandHandlers.clear();
}
