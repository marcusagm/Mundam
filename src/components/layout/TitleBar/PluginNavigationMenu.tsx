import { Show, type Component, type JSX } from 'solid-js';
import { Button } from '../../ui/Button';
import { DropdownMenu, type DropdownMenuItem } from '../../ui/DropdownMenu';
import '../../features/inspector/plugins/plugin-section-action-menu.css';

/** Renders a bounded menu for plugin navigation without adding a tab per contribution. */
export const PluginNavigationMenu: Component<{
    title: string;
    icon: JSX.Element;
    items: DropdownMenuItem[];
}> = properties => (
    <Show when={properties.items.length}>
        <DropdownMenu
            align="start"
            class="titlebar-plugin-menu"
            contentClass="plugin-section-action-menu"
            items={properties.items}
            trigger={expanded => (
                <Button
                    size="icon-sm"
                    variant="ghost"
                    aria-label={properties.title}
                    title={properties.title}
                    aria-haspopup="menu"
                    aria-expanded={expanded()}
                >
                    {properties.icon}
                </Button>
            )}
        />
    </Show>
);
