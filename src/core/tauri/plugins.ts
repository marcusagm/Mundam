import { invoke } from '@tauri-apps/api/core';
import type { PluginSettingsClient } from '../../lib/plugins';
import {
    PluginInspectorSectionsSchema,
    type PluginInspectorClient
} from '../../lib/pluginInspector';
import { eventBus } from '../utils/eventBus';

async function refreshInspectorAfter<Result>(operation: Promise<Result>): Promise<Result> {
    const result = await operation;
    eventBus.emit('plugins:inspector-changed', undefined);
    return result;
}

/** Loads validated inspector definitions and observes changes made through Settings. */
export const desktopPluginInspector: PluginInspectorClient = {
    load: async () =>
        PluginInspectorSectionsSchema.parse(await invoke('get_plugin_inspector_sections')),
    subscribe: onChange => eventBus.on('plugins:inspector-changed', onChange)
};

/** Main-window adapter. The settings panel owns feedback for these commands. */
export const desktopPlugins: PluginSettingsClient = {
    load: () => invoke('list_plugin_settings'),
    trust: (namespace, fingerprint) => invoke('trust_plugin_publisher', { namespace, fingerprint }),
    revoke: (namespace, fingerprint) =>
        refreshInspectorAfter(invoke('revoke_plugin_publisher', { namespace, fingerprint })),
    choose: () => invoke('choose_plugin_package'),
    install: reviewId => refreshInspectorAfter(invoke('install_reviewed_plugin', { reviewId })),
    discard: reviewId => invoke('discard_plugin_review', { reviewId }),
    reviewActivation: pluginId => invoke('review_plugin_activation', { pluginId }),
    enable: (reviewId, optionalPermissions) =>
        refreshInspectorAfter(invoke('enable_reviewed_plugin', { reviewId, optionalPermissions })),
    disable: pluginId => refreshInspectorAfter(invoke('disable_installed_plugin', { pluginId })),
    reviewRollback: pluginId => invoke('review_plugin_rollback', { pluginId }),
    remove: (pluginId, packageRevision) =>
        refreshInspectorAfter(invoke('remove_installed_plugin', { pluginId, packageRevision }))
};
