import { z } from 'zod';

const PluginInspectorFieldSchema = z.object({
    fieldId: z.string().min(1),
    label: z.string().min(1),
    kind: z.literal('checkbox'),
    defaultValue: z.boolean()
});

const NavigationLabelSchema = z
    .string()
    .min(1)
    .refine(label => [...label].length <= 80);

const PluginInspectorSectionSchema = z.object({
    pluginId: z.string().min(1),
    pluginName: z.string().min(1),
    sectionId: z.string().min(1),
    packageRevision: z.string().regex(/^[0-9a-f]{64}$/),
    title: z.string().min(1),
    viewportActionLabel: NavigationLabelSchema.optional(),
    sidebarLinkLabel: NavigationLabelSchema.optional(),
    titleBarActionLabel: NavigationLabelSchema.optional(),
    fields: z.array(PluginInspectorFieldSchema)
});

/** Runtime validation for a renderer's data-only inspector definitions. */
export const PluginInspectorSectionsSchema = z.array(PluginInspectorSectionSchema);

/** A section supplied by an approved plugin for a single selection. */
export type PluginInspectorSection = z.infer<typeof PluginInspectorSectionSchema>;

const RevisionSchema = z.string().regex(/^(0|[1-9][0-9]{0,18})$/);
const AvailableValueSchema = z.object({
    status: z.literal('available'),
    value: z.boolean(),
    revision: RevisionSchema,
    writable: z.boolean()
});
const SchemaMismatch = z.object({ status: z.literal('schemaMismatch') });

export const PluginInspectorValuesSchema = z.array(
    z.object({
        fieldId: z.string().min(1),
        state: z.discriminatedUnion('status', [AvailableValueSchema, SchemaMismatch])
    })
);
export const PluginInspectorEditSchema = z.discriminatedUnion('status', [
    z.object({ status: z.literal('saved'), value: z.boolean(), revision: RevisionSchema }),
    z.object({ status: z.literal('conflict') }),
    SchemaMismatch
]);

export type PluginInspectorFieldState = z.infer<typeof PluginInspectorValuesSchema>[number];

/** Identifies the asset and exact package displayed in the host Inspector. */
export interface PluginSectionTarget {
    assetId: string;
    pluginId: string;
    sectionId: string;
    packageRevision: string;
}

/** Framework-independent access to the active catalog's Inspector field values. */
export interface PluginInspectorDataClient {
    load(target: PluginSectionTarget): Promise<PluginInspectorFieldState[]>;
    save(edit: {
        target: PluginSectionTarget;
        fieldId: string;
        expectedRevision: string;
        value: boolean;
    }): Promise<z.infer<typeof PluginInspectorEditSchema>>;
}

/** Maps each navigation surface to its opt-in manifest declaration. */
export const pluginNavigationLabels = {
    viewport: 'viewportActionLabel',
    sidebar: 'sidebarLinkLabel',
    titleBar: 'titleBarActionLabel'
} as const;

/** Surface from which a gallery section can be revealed. */
export type PluginNavigationSurface = keyof typeof pluginNavigationLabels;

/** Qualifies a manifest's local section ID with its package identity. */
export function pluginInspectorSectionKey(section: PluginInspectorSection): string {
    return `${section.pluginId}:${section.sectionId}`;
}
