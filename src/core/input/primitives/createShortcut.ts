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
  
  // Generate a command name
  const commandName = `shortcut:${Date.now()}-${Math.random().toString(36).slice(2)}`;
  
  // Register shortcut immediately (not in effect to avoid loops)
  shortcutId = shortcutStore.register({
    name: options.name || 'Custom Shortcut',
    description: options.description,
    keys: options.keys,
    scope: options.scope || 'global',
    priority: options.priority,
    command: commandName,
    preventDefault: options.preventDefault ?? true,
    ignoreInputs: options.ignoreInputs ?? true,
    enabledWhen: () => isEnabled() && (options.enabled?.() ?? true),
    category: options.category,
    isDefault: false,
  });
  
  // Subscribe to command
  commandUnsub = onCommand(commandName, (payload: ShortcutPayload) => {
    setIsActive(true);
    options.action(null, payload);
    // Reset active state after a tick
    setTimeout(() => setIsActive(false), 100);
  });
  
  // Cleanup on unmount
  onCleanup(() => {
    if (shortcutId) {
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
      if (shortcutId) {
        shortcutStore.unregister(shortcutId);
        shortcutId = null;
      }
    },
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
    ...options,
  });
}

/**
 * Create multiple shortcuts at once
 */
export function useShortcuts(
  shortcuts: Array<{
    keys: string | string[];
    action: (event: Event | null, payload: ShortcutPayload) => void;
    name?: string;
    scope?: string;
  }>
): void {
  for (const shortcut of shortcuts) {
    createShortcut({
      keys: shortcut.keys,
      action: shortcut.action,
      name: shortcut.name,
      scope: shortcut.scope,
    });
  }
}
