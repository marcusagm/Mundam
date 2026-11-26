import { createMemo, Show, type Component } from 'solid-js';
import { Puzzle } from 'lucide-solid';
import { Button } from '../../../ui/Button';
import { DropdownMenu, type DropdownMenuItem } from '../../../ui/DropdownMenu';
import {
    pluginInspectorSectionKey,
    type PluginInspectorSection
} from '../../../../lib/pluginInspector';
import './plugin-viewport-actions.css';

/** Displays approved navigation actions in one native gallery toolbar menu. */
export const PluginViewportActions: Component<{
    sections: PluginInspectorSection[] | null | undefined;
    onRevealSection: (sectionKey: string) => void;
}> = properties => {
    const actions = createMemo<DropdownMenuItem[]>(() =>
        (properties.sections ?? []).flatMap(section => {
            if (!section.viewportActionLabel) return [];
            const key = pluginInspectorSectionKey(section);
            return [
                {
                    type: 'item',
                    id: key,
                    label: `${section.viewportActionLabel} · ${section.pluginName}`,
                    action: () => properties.onRevealSection(key)
                }
            ];
        })
    );

    return (
        <Show when={actions().length > 0}>
            <DropdownMenu
                align="end"
                contentClass="plugin-viewport-actions-menu"
                trigger={
                    <Button
                        variant="ghost"
                        size="icon-sm"
                        aria-label="Plugin actions"
                        title="Plugin actions"
                    >
                        <Puzzle size={16} aria-hidden="true" />
                    </Button>
                }
                items={actions()}
            />
        </Show>
    );
};
