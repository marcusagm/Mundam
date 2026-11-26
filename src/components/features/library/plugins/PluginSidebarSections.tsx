import { createMemo, createSignal, For, Show, type Component } from 'solid-js';
import { PanelRight, Puzzle } from 'lucide-solid';
import {
    pluginInspectorSectionKey,
    type PluginInspectorSection
} from '../../../../lib/pluginInspector';
import { Accordion, AccordionContent, AccordionHeader, AccordionItem } from '../../../ui/Accordion';
import { SidebarPanel } from '../../../ui/SidebarPanel';
import './plugin-sidebar-sections.css';

interface SidebarLink {
    key: string;
    label: string;
}

interface SidebarGroup {
    id: string;
    name: string;
    links: SidebarLink[];
}

/** Shows approved section links, grouped by plugin, in a bounded library sidebar region. */
export const PluginSidebarSections: Component<{
    sections: PluginInspectorSection[] | null | undefined;
    canOpenLinks: boolean;
    onOpenSection: (sectionKey: string) => void;
}> = properties => {
    const [expanded, setExpanded] = createSignal<string[]>([]);
    const groups = createMemo(() => {
        const byPlugin = new Map<string, SidebarGroup>();
        for (const section of properties.sections ?? []) {
            if (!section.sidebarLinkLabel) continue;
            let group = byPlugin.get(section.pluginId);
            if (!group) {
                group = { id: section.pluginId, name: section.pluginName, links: [] };
                byPlugin.set(section.pluginId, group);
            }
            group.links.push({
                key: pluginInspectorSectionKey(section),
                label: section.sidebarLinkLabel
            });
        }
        return [...byPlugin.values()];
    });

    return (
        <Show when={groups().length > 0}>
            <SidebarPanel title="Plugins" class="plugin-sidebar-sections">
                <Show when={!properties.canOpenLinks}>
                    <p class="plugin-sidebar-selection-hint">
                        Select one asset to open plugin details.
                    </p>
                </Show>
                <Accordion type="multiple" value={expanded()} onValueChange={setExpanded}>
                    <For each={groups()}>
                        {group => (
                            <AccordionItem value={group.id}>
                                <AccordionHeader title={group.name} icon={<Puzzle size={14} />} />
                                <AccordionContent>
                                    <For each={group.links}>
                                        {link => (
                                            <button
                                                type="button"
                                                class="nav-item plugin-sidebar-link"
                                                disabled={!properties.canOpenLinks}
                                                onClick={() => properties.onOpenSection(link.key)}
                                            >
                                                <PanelRight size={14} aria-hidden="true" />
                                                <span>{link.label}</span>
                                            </button>
                                        )}
                                    </For>
                                </AccordionContent>
                            </AccordionItem>
                        )}
                    </For>
                </Accordion>
            </SidebarPanel>
        </Show>
    );
};
