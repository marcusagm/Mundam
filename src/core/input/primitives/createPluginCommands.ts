import { createEffect, createMemo, onCleanup, untrack, type Accessor } from 'solid-js';
import {
    pluginCommandKey,
    pluginScreenRoute,
    type PluginCommand,
    type PluginPresentation,
    type PluginScreenRoute
} from '../../../lib/pluginPresentation';
import { onCommand } from '../dispatcher';
import { shortcutStore } from '../store/shortcutStore';

/** Registers a catalog's approved commands, with unassigned shortcuts and stable user preferences. */
export function createPluginCommands(
    presentation: Accessor<PluginPresentation | null | undefined>,
    navigate: (route: PluginScreenRoute) => void
) {
    const commands = createMemo(() => {
        const snapshot = presentation();
        const screens = new Set(snapshot?.screens.map(pluginScreenRoute));
        return new Map(
            (snapshot?.commands ?? []).flatMap(command => {
                const route = pluginScreenRoute({
                    pluginId: command.pluginId,
                    screenId: command.action.screen
                });
                return screens.has(route)
                    ? [[pluginCommandKey(command), { command, route }] as const]
                    : [];
            })
        );
    });
    const available = createMemo(() => Array.from(commands().values(), entry => entry.command));
    const run = (key: string) => {
        const entry = untrack(() => commands().get(key));
        if (entry) navigate(entry.route);
    };

    createEffect(() => {
        const snapshot = commands();
        untrack(() => {
            for (const [key, { command }] of snapshot) {
                const shortcutId = registerShortcut(key, command);
                const unsubscribe = onCommand(key, () => run(key));
                onCleanup(() => {
                    unsubscribe();
                    shortcutStore.unregister(shortcutId);
                });
            }
        });
    });

    return { available, run };
}

function registerShortcut(key: string, command: PluginCommand): string {
    return shortcutStore.register({
        id: key,
        preferenceId: key,
        command: key,
        name: `${command.title} · ${command.pluginName}`,
        keys: '',
        suggestedKeys: command.suggestedShortcut,
        scope: 'global',
        category: 'Plugins',
        priority: -10,
        ignoreInputs: true,
        yieldToInteractive: true,
        enabledWhen: () => {
            const shortcut = shortcutStore.getById(key);
            return Boolean(
                shortcut?.keys &&
                shortcutStore.detectConflicts(shortcut.normalizedKeys, key, 'global').length === 0
            );
        }
    });
}
