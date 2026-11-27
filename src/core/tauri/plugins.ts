import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import {
    PluginInspectorValuesSchema,
    PluginInspectorEditSchema,
    type PluginInspectorDataClient
} from '../../lib/pluginInspector';
import type { PluginSettingsClient } from '../../lib/plugins';
import {
    PluginPresentationSchema,
    type PluginPresentationClient
} from '../../lib/pluginPresentation';
import { eventBus } from '../utils/eventBus';

async function refreshPresentationAfter<Result>(operation: Promise<Result>): Promise<Result> {
    const result = await operation;
    eventBus.emit('plugins:presentation-changed', undefined);
    return result;
}

/** Loads validated plugin definitions and observes changes made through Settings. */
export const desktopPluginPresentation: PluginPresentationClient = {
    load: async () => PluginPresentationSchema.parse(await invoke('get_plugin_presentation')),
    subscribe: onChange => eventBus.on('plugins:presentation-changed', onChange)
};

/** Reads and edits plugin data with per-section feedback owned by the Inspector. */
export const desktopPluginInspectorData: PluginInspectorDataClient = {
    load: async target =>
        PluginInspectorValuesSchema.parse(await invoke('get_plugin_inspector_values', { target })),
    save: async edit =>
        PluginInspectorEditSchema.parse(await invoke('save_plugin_inspector_value', { edit }))
};

/** Main-window adapter. The settings panel owns feedback for these commands. */
export const desktopPlugins: PluginSettingsClient = {
    load: () => invoke('list_plugin_settings'),
    exportAnnotations: async () => {
        const path = await save({
            defaultPath: 'mundam-plugin-annotations.json',
            filters: [{ name: 'JSON', extensions: ['json'] }]
        });
        return path ? invoke<number>('export_plugin_annotations', { path }) : null;
    },
    storedData: {
        list: () => invoke('list_stored_plugin_data'),
        review: owner => invoke('review_stored_plugin_data', { owner }),
        purge: (owner, revision) =>
            refreshPresentationAfter(invoke('purge_stored_plugin_data', { owner, revision }))
    },
    trust: (namespace, fingerprint) => invoke('trust_plugin_publisher', { namespace, fingerprint }),
    revoke: (namespace, fingerprint) =>
        refreshPresentationAfter(invoke('revoke_plugin_publisher', { namespace, fingerprint })),
    choose: () => invoke('choose_plugin_package'),
    install: reviewId => refreshPresentationAfter(invoke('install_reviewed_plugin', { reviewId })),
    discard: reviewId => invoke('discard_plugin_review', { reviewId }),
    reviewActivation: pluginId => invoke('review_plugin_activation', { pluginId }),
    enable: (reviewId, optionalPermissions) =>
        refreshPresentationAfter(
            invoke('enable_reviewed_plugin', { reviewId, optionalPermissions })
        ),
    disable: pluginId => refreshPresentationAfter(invoke('disable_installed_plugin', { pluginId })),
    reviewRollback: pluginId => invoke('review_plugin_rollback', { pluginId }),
    remove: (pluginId, packageRevision) =>
        refreshPresentationAfter(invoke('remove_installed_plugin', { pluginId, packageRevision }))
};
