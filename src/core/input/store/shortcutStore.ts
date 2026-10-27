/**
 * Shortcut Store
 * Reactive state for registered shortcuts
 */

import { createSignal, createEffect } from 'solid-js';
import { invokeCommand as invoke } from '../../../lib/api';
import type {
    ShortcutDefinition,
    RegisteredShortcut,
    ShortcutActions,
    InputScopeName,
    SerializedShortcut,
    InputToken
} from '../types';
import { normalizeKeysToTokens, canonicalizeShortcut } from '../normalizer';
import { isMac } from '../utils/platform';
import { DEFAULT_SHORTCUTS } from './defaults';

function createShortcutStore() {
    const [shortcuts, setShortcuts] = createSignal<Map<string, RegisteredShortcut>>(new Map());
    const [nextIdValue, setNextIdValue] = createSignal(1);
    const [customizations, setCustomizations] = createSignal<Map<string, string>>(new Map());
    // User keys by "name::scope"; survives unregister/register cycles of dynamic shortcuts
    const savedKeys = new Map<string, string>();

    function getStorageKey(name: string, scope?: string): string {
        return `${name}::${scope || 'global'}`;
    }

    function generateShortcutId(): string {
        const id = `sc_${nextIdValue()}`;
        setNextIdValue(previous => previous + 1);
        return id;
    }

    function joinKeys(keys: string | string[]): string {
        return typeof keys === 'string' ? keys : keys.join(' ');
    }

    function resolveCustomKeys(definition: ShortcutDefinition, currentId: string) {
        return (
            customizations().get(currentId) ??
            savedKeys.get(getStorageKey(definition.name, definition.scope))
        );
    }

    function calculateIgnoreInputs(definition: ShortcutDefinition, tokens: InputToken[]): boolean {
        if (definition.ignoreInputs !== undefined) return definition.ignoreInputs;

        const hasModifiers = tokens.some(token => {
            const modifiers = token.meta?.modifiers;
            return Array.isArray(modifiers) && modifiers.length > 0;
        });

        return !hasModifiers;
    }

    function createRegisteredShortcut(
        definition: ShortcutDefinition,
        handler?: ShortcutDefinition['handler']
    ): RegisteredShortcut {
        const currentId = definition.id || generateShortcutId();
        const finalHandler = handler || definition.handler;
        const rawDefaultKeys =
            (definition as Partial<RegisteredShortcut>).defaultKeys ?? joinKeys(definition.keys);
        const defaultKeys = canonicalizeShortcut(rawDefaultKeys);
        const customKeys = resolveCustomKeys(definition, currentId);
        const finalKeys = customKeys ? canonicalizeShortcut(customKeys) : defaultKeys;
        const tokens = normalizeKeysToTokens(finalKeys);

        return {
            ...definition,
            id: currentId,
            handler: finalHandler,
            keys: finalKeys,
            defaultKeys,
            tokens,
            normalizedKeys: canonicalizeShortcut(finalKeys),
            scope: definition.scope || 'global',
            priority: definition.priority ?? 0,
            ignoreInputs: calculateIgnoreInputs(definition, tokens),
            preventDefault: definition.preventDefault ?? true,
            isDefault: customKeys === undefined ? (definition.isDefault ?? true) : false
        };
    }

    async function saveShortcutsToBackend() {
        try {
            await invoke('set_setting', { key: 'shortcuts', value: Object.fromEntries(savedKeys) });
        } catch (error) {
            console.warn('[ShortcutStore] Failed to save shortcuts:', error);
        }
    }

    const actions: ShortcutActions = {
        register: (definition: ShortcutDefinition, handler?: ShortcutDefinition['handler']) => {
            const registered = createRegisteredShortcut(definition, handler);

            setShortcuts(previous => {
                const next = new Map(previous);
                next.set(registered.id, registered);
                return next;
            });

            return registered.id;
        },

        unregister: (id: string) => {
            setShortcuts(previous => {
                const next = new Map(previous);
                next.delete(id);
                return next;
            });
        },

        edit: (id: string, newKeys: string, persist = true) => {
            const current = shortcuts().get(id);
            if (!current) {
                console.warn(`[ShortcutStore] Cannot edit: shortcut ${id} not found`);
                return;
            }

            setCustomizations(previous => {
                const next = new Map(previous);
                next.set(id, newKeys);
                return next;
            });

            const tokens = normalizeKeysToTokens(newKeys);
            const updated: RegisteredShortcut = {
                ...current,
                keys: newKeys,
                tokens,
                normalizedKeys: canonicalizeShortcut(newKeys),
                isDefault: false
            };

            savedKeys.set(getStorageKey(current.name, current.scope), newKeys);

            setShortcuts(previous => {
                const next = new Map(previous);
                next.set(id, updated);
                return next;
            });

            if (persist) saveShortcutsToBackend();
        },

        resetToDefault: (id: string) => {
            const current = shortcuts().get(id);
            if (!current) {
                console.warn(`[ShortcutStore] Cannot reset: shortcut ${id} not found`);
                return;
            }

            savedKeys.delete(getStorageKey(current.name, current.scope));
            setCustomizations(previous => {
                const next = new Map(previous);
                next.delete(id);
                return next;
            });

            const registered = createRegisteredShortcut({
                ...current,
                keys: current.defaultKeys,
                isDefault: true
            });
            setShortcuts(previous => {
                const next = new Map(previous);
                next.set(id, registered);
                return next;
            });

            saveShortcutsToBackend();
        },

        resetAllToDefaults: () => {
            savedKeys.clear();
            setCustomizations(new Map());

            setShortcuts(previous => {
                const next = new Map<string, RegisteredShortcut>();
                for (const current of previous.values()) {
                    next.set(
                        current.id,
                        createRegisteredShortcut({
                            ...current,
                            keys: current.defaultKeys,
                            isDefault: true
                        })
                    );
                }
                return next;
            });
            saveShortcutsToBackend();
        },

        list: () => {
            return Array.from(shortcuts().values());
        },

        getByScope: (scope: InputScopeName) => {
            return Array.from(shortcuts().values()).filter(shortcut => shortcut.scope === scope);
        },

        detectConflicts: (keys: string, excludeId?: string, scope?: string) => {
            const normalized = canonicalizeShortcut(keys);
            const targetScope = scope || 'global';

            return Array.from(shortcuts().values())
                .filter(shortcut => {
                    if (shortcut.id === excludeId) return false;
                    if (shortcut.normalizedKeys !== normalized) return false;
                    const shortcutScope = shortcut.scope || 'global';
                    return shortcutScope === targetScope;
                })
                .map(shortcut => shortcut.name);
        }
    };

    // Initialization
    for (const definition of DEFAULT_SHORTCUTS) {
        actions.register(definition);
    }

    async function loadShortcutsFromBackend() {
        try {
            const saved = await invoke<Record<string, string> | null>('get_setting', {
                key: 'shortcuts'
            });
            if (saved) {
                for (const [key, rawKeys] of Object.entries(saved)) {
                    const [name, scope] = key.split('::');
                    const keys =
                        !isMac() && rawKeys.includes('Meta')
                            ? rawKeys.replace(/\bMeta\b/g, 'Ctrl')
                            : rawKeys;
                    savedKeys.set(key, keys);
                    const found = Array.from(shortcuts().values()).find(
                        shortcut =>
                            shortcut.name === name &&
                            (shortcut.scope || 'global') === (scope || 'global')
                    );
                    if (found) {
                        actions.edit(found.id, keys, false);
                    }
                }
            }
        } catch (error) {
            console.warn('[ShortcutStore] Failed to load shortcuts:', error);
        }
    }

    createEffect(() => {
        loadShortcutsFromBackend();
    });

    return {
        shortcuts,
        customizations,
        ...actions,
        getById: (id: string) => shortcuts().get(id),
        getByCommand: (command: string) =>
            Array.from(shortcuts().values()).find(s => s.command === command),
        getByNameAndScope: (name: string, scope: InputScopeName = 'global') =>
            Array.from(shortcuts().values()).find(
                s => s.name === name && (s.scope || 'global') === (scope || 'global')
            ),
        getCategories: () => {
            const categories = new Set<string>();
            for (const shortcut of shortcuts().values()) {
                if (shortcut.category) categories.add(shortcut.category);
            }
            return Array.from(categories).sort();
        },
        serialize: (): SerializedShortcut[] =>
            Array.from(shortcuts().values()).map(shortcut => ({
                id: shortcut.id,
                name: shortcut.name,
                description: shortcut.description,
                keys: shortcut.keys as string,
                scope: shortcut.scope || 'global',
                category: shortcut.category,
                isCustom: !shortcut.isDefault
            })),
        getDefault: (id: string): ShortcutDefinition | undefined => {
            const current = shortcuts().get(id);
            if (!current) return undefined;
            return { ...current, keys: current.defaultKeys };
        }
    };
}

export const shortcutStore = createShortcutStore();
