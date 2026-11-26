import { invoke } from '@tauri-apps/api/core';
import type { PluginSettingsClient } from '../../lib/plugins';

/** Main-window adapter. The settings panel owns feedback for these commands. */
export const desktopPlugins: PluginSettingsClient = {
    load: () => invoke('list_plugin_settings'),
    trust: (namespace, fingerprint) => invoke('trust_plugin_publisher', { namespace, fingerprint }),
    revoke: (namespace, fingerprint) =>
        invoke('revoke_plugin_publisher', { namespace, fingerprint }),
    choose: () => invoke('choose_plugin_package'),
    install: reviewId => invoke('install_reviewed_plugin', { reviewId }),
    discard: reviewId => invoke('discard_plugin_review', { reviewId }),
    reviewActivation: pluginId => invoke('review_plugin_activation', { pluginId }),
    enable: (reviewId, optionalPermissions) =>
        invoke('enable_reviewed_plugin', { reviewId, optionalPermissions }),
    disable: pluginId => invoke('disable_installed_plugin', { pluginId }),
    reviewRollback: pluginId => invoke('review_plugin_rollback', { pluginId }),
    remove: (pluginId, packageRevision) =>
        invoke('remove_installed_plugin', { pluginId, packageRevision })
};
