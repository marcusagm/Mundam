import { Component, For } from 'solid-js';
import { formatShortcutForDisplay } from '../../../core/input/normalizer';
import { SectionGroup } from '../../ui/SectionGroup';
import { Kbd } from '../../ui';

const MOUSE_INTERACTIONS = [
    {
        name: 'Select an asset range',
        description: 'Select assets from the last selected item to the clicked item.',
        keys: ['Shift+Click']
    },
    {
        name: 'Add or remove an asset',
        description: 'Keep the current selection while toggling the clicked asset.',
        keys: ['Mod+Click']
    },
    {
        name: 'Combine quick filters',
        description: 'Add a quick filter without clearing the other active quick filters.',
        keys: ['Mod+Click']
    },
    {
        name: 'Filter history',
        description: 'Move backward or forward in history.',
        keys: ['Mouse Back', 'Mouse Forward']
    }
];

export const MouseInteractionsReference: Component = () => (
    <SectionGroup
        title="Mouse Interactions"
        description="These built-in mouse actions are listed for reference and are not editable as keyboard shortcuts."
    >
        <For each={MOUSE_INTERACTIONS}>
            {interaction => (
                <div class="shortcut-row">
                    <div class="shortcut-info">
                        <span class="shortcut-name">{interaction.name}</span>
                        <span class="shortcut-description">{interaction.description}</span>
                    </div>
                    <div class="shortcut-keys-container">
                        <For each={interaction.keys}>
                            {key => (
                                <Kbd>
                                    {key.includes('Click') ? formatShortcutForDisplay(key) : key}
                                </Kbd>
                            )}
                        </For>
                    </div>
                </div>
            )}
        </For>
    </SectionGroup>
);
