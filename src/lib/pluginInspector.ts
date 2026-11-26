import { z } from 'zod';

const PluginInspectorFieldSchema = z.object({
    fieldId: z.string().min(1),
    label: z.string().min(1),
    kind: z.literal('checkbox'),
    defaultValue: z.boolean()
});

const PluginInspectorSectionSchema = z.object({
    pluginId: z.string().min(1),
    pluginName: z.string().min(1),
    sectionId: z.string().min(1),
    title: z.string().min(1),
    viewportActionLabel: z
        .string()
        .min(1)
        .refine(label => [...label].length <= 80)
        .optional(),
    fields: z.array(PluginInspectorFieldSchema)
});

/** Runtime validation for a renderer's data-only inspector definitions. */
export const PluginInspectorSectionsSchema = z.array(PluginInspectorSectionSchema);

/** A section supplied by an approved plugin for a single selection. */
export type PluginInspectorSection = z.infer<typeof PluginInspectorSectionSchema>;

/** Qualifies a manifest's local section ID with its package identity. */
export function pluginInspectorSectionKey(section: PluginInspectorSection): string {
    return `${section.pluginId}:${section.sectionId}`;
}

/** Framework-independent client for reading and refreshing approved sections. */
export interface PluginInspectorClient {
    load(): Promise<PluginInspectorSection[]>;
    subscribe(onChange: () => void): () => void;
}
