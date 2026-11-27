import { createResource, createSignal, onCleanup, Show, untrack, type Component } from 'solid-js';
import type {
    InstalledPlugin,
    PluginActivationReview,
    PluginPackageReview as PackageReview,
    PluginPermission,
    PluginSettingsClient
} from '../../../../lib/plugins';
import { Button } from '../../../ui/Button';
import { SectionGroup } from '../../../ui/SectionGroup';
import { PluginPackageReview } from './PluginPackageReview';
import { PluginActivationConsent } from './PluginActivationConsent';
import { PluginDataSection } from './PluginDataSection';
import { InstalledPluginList } from './InstalledPluginList';
import { PluginPublisherSection } from './PluginPublisherSection';
import './plugins-panel.css';

/** Reviews and installs locally signed packages through an injectable settings client. */
export const PluginsPanel: Component<{ client: PluginSettingsClient }> = properties => {
    const [settings, { refetch }] = createResource(() =>
        properties.client.load().catch(() => null)
    );
    const [review, setReview] = createSignal<PackageReview>();
    const [activation, setActivation] = createSignal<PluginActivationReview>();
    const [busy, setBusy] = createSignal(false);
    const [status, setStatus] = createSignal('');
    const [failure, setFailure] = createSignal('');
    const [packageFailure, setPackageFailure] = createSignal('');
    const [activationFailure, setActivationFailure] = createSignal('');
    let disposed = false;

    const discard = (reviewId: string | null | undefined) => {
        if (reviewId) void properties.client.discard(reviewId).catch(() => undefined);
    };

    onCleanup(() => {
        disposed = true;
        discard(review()?.reviewId);
        discard(untrack(activation)?.reviewId);
    });

    const runAction = async (
        action: () => Promise<void>,
        failureMessage: string,
        showFailure: (message: string) => void = setFailure
    ) => {
        if (busy()) return false;
        setBusy(true);
        setFailure('');
        setPackageFailure('');
        setActivationFailure('');
        setStatus('');
        try {
            await action();
            return true;
        } catch {
            showFailure(failureMessage);
            return false;
        } finally {
            setBusy(false);
        }
    };

    const cancelActivation = () => {
        discard(untrack(activation)?.reviewId);
        setActivation(undefined);
    };

    const choose = () =>
        runAction(
            async () => {
                cancelActivation();
                setReview(undefined);
                const chosen = await properties.client.choose();
                if (disposed) {
                    discard(chosen?.reviewId);
                    return;
                }
                setReview(chosen ?? undefined);
            },
            'Could not review this package. Choose a valid signed package from a trusted publisher.',
            setPackageFailure
        );

    const install = (reviewId: string) =>
        runAction(
            async () => {
                const replacement = untrack(review)?.replacement;
                setReview(undefined);
                const installed = await properties.client.install(reviewId);
                if (disposed) return;
                if (replacement) {
                    setStatus(
                        `${installed.name} ${installed.version} is installed and disabled in all catalogs.`
                    );
                } else {
                    setStatus(`${installed.name} is installed.`);
                }
                await refetch();
            },
            'Could not install this review. Choose the package again.',
            setPackageFailure
        );

    const reviewRollback = (pluginId: string) =>
        runAction(
            async () => {
                cancelActivation();
                discard(untrack(review)?.reviewId);
                setReview(undefined);
                const pending = await properties.client.reviewRollback(pluginId);
                if (disposed) {
                    discard(pending.reviewId);
                    return;
                }
                setReview(pending);
            },
            'Could not review the previous version. Check its compatibility and publisher trust.',
            setActivationFailure
        );

    const remove = (plugin: InstalledPlugin) =>
        runAction(
            async () => {
                cancelActivation();
                discard(untrack(review)?.reviewId);
                setReview(undefined);
                try {
                    await properties.client.remove(plugin.id, plugin.packageRevision);
                } finally {
                    await refetch();
                }
                setStatus(`${plugin.name} was removed from this profile.`);
            },
            'Could not remove this package. Review the installed version and try again.',
            setActivationFailure
        );

    const reviewActivation = (pluginId: string) =>
        runAction(
            async () => {
                cancelActivation();
                discard(untrack(review)?.reviewId);
                setReview(undefined);
                const pending = await properties.client.reviewActivation(pluginId);
                if (disposed) {
                    discard(pending.reviewId);
                    return;
                }
                setActivation(pending);
            },
            'Could not review these permissions. Check the package and publisher trust.',
            setActivationFailure
        );

    const enable = (reviewId: string, optionalPermissions: PluginPermission[]) =>
        runAction(
            async () => {
                setActivation(undefined);
                await properties.client.enable(reviewId, optionalPermissions);
                await refetch();
                setStatus('Plugin enabled for this catalog.');
            },
            'Could not enable this plugin. Review its permissions again.',
            setActivationFailure
        );

    const disable = (pluginId: string) =>
        runAction(
            async () => {
                cancelActivation();
                await properties.client.disable(pluginId);
                await refetch();
                setStatus('Plugin permissions revoked for this catalog.');
            },
            'Could not disable this plugin.',
            setActivationFailure
        );

    return (
        <div class="settings-panel-content">
            <h2 class="settings-panel-title">Plugins</h2>
            <p class="settings-panel-description">
                Manage trusted publishers and install signed plugins from a file.
            </p>
            <Show
                when={settings()}
                fallback={
                    <Show
                        when={settings.loading}
                        fallback={
                            <div class="plugin-settings-actions">
                                <p role="alert">Could not load plugin settings.</p>
                                <Button size="sm" variant="outline" onClick={() => void refetch()}>
                                    Retry
                                </Button>
                            </div>
                        }
                    >
                        <p role="status">Loading plugin settings…</p>
                    </Show>
                }
            >
                {current => (
                    <>
                        <PluginPublisherSection
                            publishers={current().publishers}
                            busy={busy()}
                            onTrust={(namespace, fingerprint) =>
                                runAction(async () => {
                                    await properties.client.trust(namespace, fingerprint);
                                    setStatus('Publisher trusted.');
                                    await refetch();
                                }, 'Could not trust this publisher. Check the identity and any existing registration.')
                            }
                            onRevoke={publisher =>
                                runAction(async () => {
                                    await properties.client.revoke(
                                        publisher.namespace,
                                        publisher.fingerprint
                                    );
                                    setStatus('Publisher trust revoked.');
                                    await refetch();
                                }, 'Could not revoke publisher trust.')
                            }
                        />
                        <SectionGroup
                            title="Install from file"
                            description="Choose a signed package to install or update a plugin."
                        >
                            <div class="plugin-settings-section">
                                <div class="plugin-settings-actions">
                                    <Button
                                        size="sm"
                                        variant="outline"
                                        disabled={busy()}
                                        onClick={() => void choose()}
                                    >
                                        Choose plugin package
                                    </Button>
                                </div>
                                <Show when={packageFailure()}>
                                    <p role="alert">{packageFailure()}</p>
                                </Show>
                                <Show when={review()} keyed>
                                    {currentReview => (
                                        <PluginPackageReview
                                            review={currentReview}
                                            busy={busy()}
                                            onInstall={reviewId => void install(reviewId)}
                                            onCancel={() => {
                                                discard(currentReview.reviewId);
                                                setReview(undefined);
                                            }}
                                        />
                                    )}
                                </Show>
                            </div>
                        </SectionGroup>
                        <SectionGroup title="Installed plugins">
                            <div class="plugin-settings-section">
                                <InstalledPluginList
                                    settings={current()}
                                    busy={busy()}
                                    onReview={pluginId => void reviewActivation(pluginId)}
                                    onDisable={pluginId => void disable(pluginId)}
                                    onRollback={pluginId => void reviewRollback(pluginId)}
                                    onRemove={plugin => void remove(plugin)}
                                />
                                <Show when={activationFailure()}>
                                    <p role="alert">{activationFailure()}</p>
                                </Show>
                                <Show when={activation()} keyed>
                                    {pending => (
                                        <PluginActivationConsent
                                            review={pending}
                                            busy={busy()}
                                            onEnable={(id, optionalPermissions) =>
                                                void enable(id, optionalPermissions)
                                            }
                                            onCancel={cancelActivation}
                                        />
                                    )}
                                </Show>
                            </div>
                        </SectionGroup>
                        <Show when={failure()}>
                            <p role="alert">{failure()}</p>
                        </Show>
                        <Show when={status()}>
                            <p role="status">{status()}</p>
                        </Show>
                    </>
                )}
            </Show>
            <PluginDataSection exportAnnotations={() => properties.client.exportAnnotations()} />
        </div>
    );
};
