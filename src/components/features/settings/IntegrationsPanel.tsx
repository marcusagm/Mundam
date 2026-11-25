import { createResource, createSignal, Show, type Component } from 'solid-js';
import type { ExternalImportClient } from '../../../lib/externalImport';
import { Button } from '../../ui/Button';
import { SectionGroup } from '../../ui/SectionGroup';
import { ImportPairingPanel } from './ImportPairingPanel';
import './integrations-panel.css';

/** Starts and stops the local import API for the current desktop session. */
export const IntegrationsPanel: Component<{
    externalImport: ExternalImportClient;
}> = properties => {
    const [status, { mutate, refetch }] = createResource(() =>
        properties.externalImport.status().catch(() => null)
    );
    const [busy, setBusy] = createSignal(false);
    const [message, setMessage] = createSignal('');

    const toggle = async () => {
        const current = status();
        if (!current?.available || busy()) return;
        setBusy(true);
        setMessage('');
        try {
            if (current.endpoint) {
                await properties.externalImport.stop();
                mutate({ ...current, endpoint: null });
            } else {
                const endpoint = await properties.externalImport.start();
                mutate({ ...current, endpoint });
            }
        } catch {
            setMessage(
                current.endpoint
                    ? 'Could not disable the local import API.'
                    : 'Could not enable the local import API.'
            );
        } finally {
            setBusy(false);
        }
    };

    return (
        <div class="settings-panel-content">
            <h2 class="settings-panel-title">Integrations</h2>
            <p class="settings-panel-description">
                Control local connections from other applications.
            </p>
            <SectionGroup
                title="Local import API"
                description="Enable connections for this Mundam session."
            >
                <div class="local-import-settings">
                    <Show
                        when={!status.loading}
                        fallback={<p role="status">Loading integrations…</p>}
                    >
                        <Show
                            when={status()}
                            fallback={
                                <div class="local-import-actions">
                                    <p role="alert">Could not load integration settings.</p>
                                    <Button
                                        size="sm"
                                        variant="outline"
                                        onClick={() => void refetch()}
                                    >
                                        Retry
                                    </Button>
                                </div>
                            }
                        >
                            {current => (
                                <Show
                                    when={current().available}
                                    fallback={
                                        <p>The local import API is unavailable on this platform.</p>
                                    }
                                >
                                    <div class="local-import-actions">
                                        <p role="status">
                                            {current().endpoint ? 'Enabled' : 'Disabled'}
                                        </p>
                                        <Button
                                            size="sm"
                                            variant="outline"
                                            loading={busy()}
                                            onClick={() => void toggle()}
                                        >
                                            {current().endpoint ? 'Disable API' : 'Enable API'}
                                        </Button>
                                    </div>
                                    <Show when={current().endpoint}>
                                        {endpoint => (
                                            <dl>
                                                <dt>Local address</dt>
                                                <dd>
                                                    <code>{endpoint().url}</code>
                                                </dd>
                                                <dt>Public key fingerprint (SHA-256)</dt>
                                                <dd>
                                                    <code>{endpoint().publicKeySha256}</code>
                                                </dd>
                                            </dl>
                                        )}
                                    </Show>
                                    <Show when={message()}>
                                        <p role="alert">{message()}</p>
                                    </Show>
                                </Show>
                            )}
                        </Show>
                    </Show>
                </div>
            </SectionGroup>
            <Show when={status()?.endpoint}>
                <ImportPairingPanel pairing={properties.externalImport.pairing} disabled={busy()} />
            </Show>
        </div>
    );
};
