import { createMemo, type Component } from 'solid-js';
import { Zap } from 'lucide-solid';
import { pluginCommandKey, type PluginCommand } from '../../../lib/pluginPresentation';
import type { DropdownMenuItem } from '../../ui/DropdownMenu';
import { PluginNavigationMenu } from './PluginNavigationMenu';

/** Exposes approved commands even when no shortcuts are assigned. */
export const PluginCommandsMenu: Component<{
    commands: PluginCommand[];
    onRun: (key: string) => void;
}> = properties => {
    const items = createMemo<DropdownMenuItem[]>(() =>
        properties.commands.map(command => {
            const key = pluginCommandKey(command);
            return {
                type: 'item',
                id: key,
                label: `${command.title} · ${command.pluginName}`,
                action: () => properties.onRun(key)
            };
        })
    );
    return (
        <PluginNavigationMenu
            title="Plugin commands"
            icon={<Zap size={16} aria-hidden="true" />}
            items={items()}
        />
    );
};
