import { z } from 'zod';
import { PluginInspectorSectionsSchema } from './pluginInspector';

const ScreenBlockSchema = z.discriminatedUnion('kind', [
    z.strictObject({ kind: z.literal('paragraph'), text: z.string().min(1) }),
    z.strictObject({ kind: z.literal('list'), items: z.array(z.string().min(1)).min(1).max(12) })
]);

const PluginScreenSchema = z.object({
    pluginId: z.string().min(1),
    pluginName: z.string().min(1),
    screenId: z.string().regex(/^[a-z][a-z0-9-]{0,31}$/),
    title: z.string().min(1),
    blocks: z.array(ScreenBlockSchema).min(1).max(12)
});

const PluginCommandSchema = z.object({
    pluginId: z.string().min(1),
    pluginName: z.string().min(1),
    commandId: z.string().regex(/^[a-z][a-z0-9-]{0,31}$/),
    title: z.string().min(1),
    suggestedShortcut: z
        .string()
        .regex(/^Mod\+(Shift\+)?Key[A-Z]$/)
        .optional(),
    action: z.strictObject({
        kind: z.literal('openScreen'),
        screen: z.string().regex(/^[a-z][a-z0-9-]{0,31}$/)
    })
});

/** Catalog-scoped declarations sent to the host renderer in one response. */
export const PluginPresentationSchema = z.object({
    sections: PluginInspectorSectionsSchema,
    screens: z.array(PluginScreenSchema),
    commands: z.array(PluginCommandSchema)
});

export type PluginPresentation = z.infer<typeof PluginPresentationSchema>;
export type PluginScreen = z.infer<typeof PluginScreenSchema>;
export type PluginCommand = z.infer<typeof PluginCommandSchema>;
export type PluginScreenRoute = `plugin:${string}:${string}`;

/** Returns the application route for a qualified screen ID. */
export function pluginScreenRoute(
    screen: Pick<PluginScreen, 'pluginId' | 'screenId'>
): PluginScreenRoute {
    return `plugin:${screen.pluginId}:${screen.screenId}`;
}

/** Returns a reserved host command name for a qualified plugin action. */
export function pluginCommandKey(command: PluginCommand): string {
    return `plugin-command:${command.pluginId}:${command.commandId}`;
}

/** Framework-independent source of approved plugin definitions. */
export interface PluginPresentationClient {
    load(): Promise<PluginPresentation>;
    subscribe(onChange: () => void): () => void;
}
