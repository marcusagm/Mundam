import { createEffect, createSignal, For, Show, untrack, type Component } from 'solid-js';
import { Puzzle } from 'lucide-solid';
import {
    pluginInspectorSectionKey,
    type PluginInspectorDataClient,
    type PluginInspectorSection
} from '../../../../lib/pluginInspector';
import type { PluginPresentationState } from '../../../../core/hooks/createPluginPresentation';
import { Accordion, AccordionContent, AccordionHeader, AccordionItem } from '../../../ui/Accordion';
import { Button } from '../../../ui/Button';
import { PluginInspectorFields } from './PluginInspectorFields';
import './plugin-inspector-sections.css';

interface SectionRevealProperties {
    revealSection?: string;
    onSectionRevealed?: () => void;
}

interface SectionDataProperties {
    assetId: string;
    client: PluginInspectorDataClient;
}

const InspectorAccordion: Component<
    SectionRevealProperties &
        SectionDataProperties & {
            sections: PluginInspectorSection[];
        }
> = properties => {
    const [expanded, setExpanded] = createSignal(
        untrack(() => properties.sections.map(pluginInspectorSectionKey))
    );

    return (
        <Accordion type="multiple" value={expanded()} onValueChange={setExpanded}>
            <For each={properties.sections}>
                {section => {
                    const key = pluginInspectorSectionKey(section);
                    let trigger: HTMLButtonElement | undefined;
                    createEffect(() => {
                        if (properties.revealSection !== key || !trigger) return;
                        setExpanded(current =>
                            current.includes(key) ? current : [...current, key]
                        );
                        trigger.focus();
                        properties.onSectionRevealed?.();
                    });
                    return (
                        <AccordionItem value={key}>
                            <AccordionHeader
                                title={section.title}
                                icon={<Puzzle size={14} />}
                                ref={element => {
                                    trigger = element;
                                }}
                            />
                            <AccordionContent>
                                <Show when={properties.assetId} keyed>
                                    {assetId => (
                                        <PluginInspectorFields
                                            section={section}
                                            assetId={assetId}
                                            client={properties.client}
                                        />
                                    )}
                                </Show>
                            </AccordionContent>
                        </AccordionItem>
                    );
                }}
            </For>
        </Accordion>
    );
};

/** Renders approved definitions as native inspector controls for the current asset. */
export const PluginInspectorSections: Component<
    SectionRevealProperties &
        SectionDataProperties & {
            presentation: PluginPresentationState;
        }
> = properties => {
    const sections = () => properties.presentation.sections();
    return (
        <>
            <Show when={sections() === null}>
                <div class="plugin-inspector-feedback">
                    <p role="status">Could not load plugin sections.</p>
                    <Button
                        size="xs"
                        variant="outline"
                        onClick={() => properties.presentation.refresh()}
                    >
                        Retry
                    </Button>
                </div>
            </Show>
            <Show when={sections()} keyed>
                {available =>
                    available.length > 0 && (
                        <InspectorAccordion
                            sections={available}
                            assetId={properties.assetId}
                            client={properties.client}
                            revealSection={properties.revealSection}
                            onSectionRevealed={properties.onSectionRevealed}
                        />
                    )
                }
            </Show>
        </>
    );
};
