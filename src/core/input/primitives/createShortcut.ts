/**
 * createShortcut Primitive
 * Main API for registering shortcuts in components
 */

import { onCleanup, createSignal, Accessor } from 'solid-js';
import { shortcutStore } from '../store/shortcutStore';
import { onCommand } from '../dispatcher';
import type { CreateShortcutOptions, ShortcutPayload } from '../types';

// =============================================================================
// Main Primitive
// =============================================================================

interface ShortcutHandle {
    /** Whether this shortcut is currently active */
    isActive: Accessor<boolean>;
    /** Temporarily disable this shortcut */
    disable: () => void;
    /** Re-enable this shortcut */
    enable: () => void;
    /** Unregister this shortcut */
    unregister: () => void;
}

/**
 * Create and register a keyboard shortcut
 *
 * @example
 * // Basic usage
 * createShortcut({
 *   keys: 'Meta+KeyS',
 *   action: () => saveDocument(),
 * });
 *
 * @example
 * // With options
 * const shortcut = createShortcut({
 *   keys: 'Meta+KeyK',
 *   action: (e) => openSearch(),
 *   scope: 'global',
 *   ignoreInputs: false, // Works even in inputs
 * });
 *
 * // Later, disable/enable
 * shortcut.disable();
 * shortcut.enable();
 */
export function createShortcut(options: CreateShortcutOptions): ShortcutHandle {
    const [isEnabled, setIsEnabled] = createSignal(true);
    const [isActive, setIsActive] = createSignal(false);

    let shortcutId: string | null = null;
    let commandUnsub: (() => void) | null = null;

    // Check if shortcut already exists (to reuse definition/customization)
    // Type assertion since getByNameAndScope is newly added and TS might not see it yet in inference or if interface wasn't updated fully
    const existing = options.name
        ? shortcutStore.getByNameAndScope(options.name, options.scope)
        : undefined;

    let commandName: string;
    let wasCreated = false;

    if (existing) {
        // Reuse existing shortcut
        shortcutId = existing.id;
        // Use existing command or generate one if missing (shouldn't happen for active shortcuts)
        commandName =
            existing.command || `shortcut:${Date.now()}-${Math.random().toString(36).slice(2)}`;

        // If we're attaching to an existing shortcut, we might want to ensure it has a command
        if (!existing.command) {
            // Technically we should update the store here, but let's assume valid state for now
            // or re-register to update command
            console.warn(`[createShortcut] Existing shortcut ${existing.name} has no command`);
        }
    } else {
        // Generate a command name
        commandName = `shortcut:${Date.now()}-${Math.random().toString(36).slice(2)}`;

        // Register shortcut immediately
        wasCreated = true;
        shortcutId = shortcutStore.register({
            name: options.name || 'Custom Shortcut',
            description: options.description,
            keys: options.keys,
            scope: options.scope || 'global',
            priority: options.priority,
            command: commandName,
            preventDefault: options.preventDefault ?? true,
            ignoreInputs: options.ignoreInputs ?? true,
            allowOnInteractive: options.allowOnInteractive,
            enabledWhen: () => isEnabled() && (options.enabled?.() ?? true),
            category: options.category,
            system: options.system,
            isDefault: false
        });
    }

    // Subscribe to command
    commandUnsub = onCommand(commandName, (payload: ShortcutPayload) => {
        // Check if locally enabled before executing
        if (options.enabled && !options.enabled()) {
            return;
        }

        setIsActive(true);
        options.action(null, payload);
        // Reset active state after a tick
        setTimeout(() => setIsActive(false), 100);
    });

    // Cleanup on unmount
    onCleanup(() => {
        if (shortcutId && wasCreated) {
            shortcutStore.unregister(shortcutId);
        }
        if (commandUnsub) {
            commandUnsub();
        }
    });

    return {
        isActive,
        disable: () => setIsEnabled(false),
        enable: () => setIsEnabled(true),
        unregister: () => {
            if (shortcutId && wasCreated) {
                shortcutStore.unregister(shortcutId);
                shortcutId = null;
            }
        }
    };
}

// =============================================================================
// Convenience Wrappers
// =============================================================================

/**
 * Simple shortcut without handle return
 * Use when you don't need to disable/enable later
 */
export function useShortcut(
    keys: string | string[],
    action: (event: Event | null, payload: ShortcutPayload) => void,
    options?: Partial<Omit<CreateShortcutOptions, 'keys' | 'action'>>
): void {
    createShortcut({
        keys,
        action,
        ...options
    });
}

/**
 * Create multiple shortcuts at once
 */
export function useShortcuts(shortcuts: CreateShortcutOptions[]): void {
    for (const shortcut of shortcuts) {
        createShortcut(shortcut);
    }
}
