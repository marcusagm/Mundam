import { createMemo, Show, type Component } from 'solid-js';
import { PanelsTopLeft } from 'lucide-solid';
import {
    pluginScreenRoute,
    type PluginScreen,
    type PluginScreenRoute
} from '../../../lib/pluginPresentation';
import { Button } from '../../ui/Button';
import { DropdownMenu, type DropdownMenuItem } from '../../ui/DropdownMenu';
import '../../features/inspector/plugins/plugin-section-action-menu.css';

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
        <Show when={items().length}>
            <DropdownMenu
                align="start"
                class="titlebar-plugin-screens"
                contentClass="plugin-section-action-menu"
                items={items()}
                trigger={expanded => (
                    <Button
                        size="icon-sm"
                        variant="ghost"
                        aria-label="Plugin screens"
                        title="Plugin screens"
                        aria-haspopup="menu"
                        aria-expanded={expanded()}
                    >
                        <PanelsTopLeft size={16} aria-hidden="true" />
                    </Button>
                )}
            />
        </Show>
    );
};
