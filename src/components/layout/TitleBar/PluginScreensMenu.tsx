import { createMemo, type Component } from 'solid-js';
import { PanelsTopLeft } from 'lucide-solid';
import {
    pluginScreenRoute,
    type PluginScreen,
    type PluginScreenRoute
} from '../../../lib/pluginPresentation';
import type { DropdownMenuItem } from '../../ui/DropdownMenu';
import { PluginNavigationMenu } from './PluginNavigationMenu';

/** Lists every approved plugin screen in one title bar menu. */
export const PluginScreensMenu: Component<{
    screens: PluginScreen[];
    onNavigate: (route: PluginScreenRoute) => void;
}> = properties => {
    const items = createMemo<DropdownMenuItem[]>(() =>
        properties.screens.map(screen => {
            const route = pluginScreenRoute(screen);
            return {
                type: 'item',
                id: route,
                label: `${screen.title} · ${screen.pluginName}`,
                action: () => properties.onNavigate(route)
            };
        })
    );

    return (
        <PluginNavigationMenu
            title="Plugin screens"
            icon={<PanelsTopLeft size={16} aria-hidden="true" />}
            items={items()}
        />
    );
};
