import { createSignal, For, Show, type Component } from 'solid-js';
import type { InstalledPlugin, PluginSettings } from '../../../../lib/plugins';
import { Button } from '../../../ui/Button';

/** Shows the catalog's effective plugin state and its available actions. */
export const InstalledPluginList: Component<{
    settings: PluginSettings;
    busy: boolean;
    onReview(pluginId: string): void;
    onDisable(pluginId: string): void;
    onRollback(pluginId: string): void;
    onRemove(plugin: InstalledPlugin): void;
}> = properties => {
    const [removal, setRemoval] = createSignal<string>();
    return (
        <>
            <Show when={properties.settings.safeMode}>
                <p role="status">
                    Plugin safe mode is active. Enable plugins after restarting normally.
                </p>
            </Show>
            <Show
                when={properties.settings.installed.length}
                fallback={<p>No installed plugins.</p>}
            >
                <ul class="plugin-settings-list" aria-label="Installed plugins">
                    <For each={properties.settings.installed}>
                        {plugin => {
                            const trusted = () =>
                                properties.settings.publishers.some(
                                    publisher =>
                                        publisher.namespace === plugin.publisher &&
                                        !publisher.revoked
                                );
                            const canEnable = () =>
                                !plugin.approved && !properties.settings.safeMode && trusted();
                            return (
                                <li class="plugin-settings-entry">
                                    <div class="plugin-settings-actions">
                                        <strong>{plugin.name}</strong>
                                        <span>
                                            {plugin.enabled
                                                ? 'Enabled'
                                                : plugin.approved
                                                  ? 'Paused in safe mode'
                                                  : 'Disabled'}
                                        </span>
                                    </div>
                                    <p>
                                        {plugin.publisher} · {plugin.version}
                                    </p>
                                    <div class="plugin-settings-actions">
                                        <Show when={plugin.approved || canEnable()}>
                                            <Show
                                                when={plugin.approved}
                                                fallback={
                                                    <Button
                                                        size="sm"
                                                        variant="outline"
                                                        disabled={properties.busy}
                                                        onClick={() =>
                                                            properties.onReview(plugin.id)
                                                        }
                                                    >
                                                        Enable in this catalog
                                                    </Button>
                                                }
                                            >
                                                <Button
                                                    size="sm"
                                                    variant="outline"
                                                    disabled={properties.busy}
                                                    onClick={() => properties.onDisable(plugin.id)}
                                                >
                                                    Disable in this catalog
                                                </Button>
                                            </Show>
                                        </Show>
                                        <Show when={trusted() && plugin.rollbackVersion}>
                                            {version => (
                                                <Button
                                                    size="sm"
                                                    variant="outline"
                                                    disabled={properties.busy}
                                                    onClick={() => {
                                                        setRemoval(undefined);
                                                        properties.onRollback(plugin.id);
                                                    }}
                                                >
                                                    Review rollback to {version()}
                                                </Button>
                                            )}
                                        </Show>
                                        <Button
                                            size="sm"
                                            variant="ghost-destructive"
                                            disabled={properties.busy}
                                            onClick={() => setRemoval(plugin.packageRevision)}
                                        >
                                            Remove plugin
                                        </Button>
                                    </div>
                                    <Show when={removal() === plugin.packageRevision}>
                                        <p>
                                            Remove {plugin.name} from this profile? This clears its
                                            package, rollback version and approvals in all catalogs.
                                        </p>
                                        <div class="plugin-settings-actions">
                                            <Button
                                                size="sm"
                                                variant="destructive"
                                                disabled={properties.busy}
                                                onClick={() => {
                                                    setRemoval(undefined);
                                                    properties.onRemove(plugin);
                                                }}
                                            >
                                                Confirm removal
                                            </Button>
                                            <Button
                                                size="sm"
                                                variant="outline"
                                                disabled={properties.busy}
                                                onClick={() => setRemoval(undefined)}
                                            >
                                                Cancel removal
                                            </Button>
                                        </div>
                                    </Show>
                                </li>
                            );
                        }}
                    </For>
                </ul>
            </Show>
        </>
    );
};
