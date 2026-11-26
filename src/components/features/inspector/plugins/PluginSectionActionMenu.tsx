import { createMemo, Show, type Component } from 'solid-js';
import { Puzzle } from 'lucide-solid';
import { Button } from '../../../ui/Button';
import { DropdownMenu, type DropdownMenuItem } from '../../../ui/DropdownMenu';
import {
    pluginInspectorSectionKey,
    pluginNavigationLabels,
    type PluginInspectorSection,
    type PluginNavigationSurface
} from '../../../../lib/pluginInspector';
import './plugin-section-action-menu.css';

/** Displays approved navigation actions in one bounded gallery menu. */
export const PluginSectionActionMenu: Component<{
    surface: Extract<PluginNavigationSurface, 'titleBar' | 'viewport'>;
    buttonLabel: string;
    sections: PluginInspectorSection[] | null | undefined;
    onRevealSection: (sectionKey: string) => void;
}> = properties => {
    const actions = createMemo<DropdownMenuItem[]>(() =>
        (properties.sections ?? []).flatMap(section => {
            const actionLabel = section[pluginNavigationLabels[properties.surface]];
            if (!actionLabel) return [];
            const key = pluginInspectorSectionKey(section);
            return [
                {
                    type: 'item',
                    id: key,
                    label: `${actionLabel} · ${section.pluginName}`,
                    action: () => properties.onRevealSection(key)
                }
            ];
        })
    );

    return (
        <Show when={actions().length > 0}>
            <DropdownMenu
                align="end"
                class="plugin-section-action-trigger"
                contentClass="plugin-section-action-menu"
                trigger={expanded => (
                    <Button
                        variant="ghost"
                        size="icon-sm"
                        aria-label={properties.buttonLabel}
                        aria-haspopup="menu"
                        aria-expanded={expanded()}
                        title={properties.buttonLabel}
                    >
                        <Puzzle size={16} aria-hidden="true" />
                    </Button>
                )}
                items={actions()}
            />
        </Show>
    );
};
