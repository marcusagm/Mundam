import { createResource, createSignal, For, Show, type Component } from 'solid-js';
import type { ExternalImportPairingClient } from '../../../lib/externalImport';
import { Button } from '../../ui/Button';
import { SectionGroup } from '../../ui/SectionGroup';
import { ImportConsentRequest, importScopeLabels } from './ImportConsentRequest';

/** Handles invitation sharing and the user's decision for each external client. */
export const ImportPairingPanel: Component<{
    pairing: ExternalImportPairingClient;
    disabled: boolean;
}> = properties => {
    const [catalog, { refetch }] = createResource(() =>
        properties.pairing.status().catch(() => null)
    );
    const [busy, setBusy] = createSignal(false);
    const [message, setMessage] = createSignal<{ text: string; failed: boolean }>();
    const disabled = () => properties.disabled || busy() || catalog.loading;

    const perform = async (
        action: () => Promise<void>,
        success: string,
        failure: string,
        reload = true
    ) => {
        if (disabled()) return;
        setBusy(true);
        setMessage(undefined);
        try {
            await action();
            setMessage({ text: success, failed: false });
            if (reload) await refetch();
        } catch {
            setMessage({ text: failure, failed: true });
        } finally {
            setBusy(false);
        }
    };

    return (
        <SectionGroup
            title="Connect an application"
            description="Review and approve each application before it can access your catalog."
        >
            <div class="local-import-settings import-pairing-settings">
                <div class="import-pairing-buttons">
                    <Button
                        size="sm"
                        variant="outline"
                        disabled={disabled()}
                        onClick={() =>
                            void perform(
                                () => properties.pairing.copyInvitation(),
                                'Invitation copied. Paste it into the application you want to connect within five minutes.',
                                'Could not copy a pairing invitation.',
                                false
                            )
                        }
                    >
                        Copy pairing invitation
                    </Button>
                    <Button
                        size="sm"
                        variant="outline"
                        disabled={disabled()}
                        onClick={() => void refetch()}
                    >
                        Refresh requests and clients
                    </Button>
                </div>
                <Show when={message()}>
                    {result => <p role={result().failed ? 'alert' : 'status'}>{result().text}</p>}
                </Show>
                <Show
                    when={catalog()}
                    fallback={
                        <Show
                            when={catalog.loading}
                            fallback={
                                <p role="alert">Could not load pairing requests and clients.</p>
                            }
                        >
                            <p role="status">Loading pairing requests…</p>
                        </Show>
                    }
                >
                    {current => (
                        <>
                            <div class="import-pairing-list">
                                <h4>Pending requests</h4>
                                <For
                                    each={current().pending}
                                    fallback={<p>No requests are waiting for approval.</p>}
                                >
                                    {request => (
                                        <ImportConsentRequest
                                            request={request}
                                            catalog={current()}
                                            disabled={disabled()}
                                            approve={approval =>
                                                void perform(
                                                    () => properties.pairing.approve(approval),
                                                    'Request approved. The application can now finish pairing.',
                                                    'Could not approve this request. It may have expired.'
                                                )
                                            }
                                            reject={requestId =>
                                                void perform(
                                                    () => properties.pairing.reject(requestId),
                                                    'Request declined.',
                                                    'Could not decline this request.'
                                                )
                                            }
                                        />
                                    )}
                                </For>
                            </div>
                            <div class="import-pairing-list">
                                <h4>Connected applications</h4>
                                <For
                                    each={current().clients}
                                    fallback={<p>No paired applications.</p>}
                                >
                                    {client => (
                                        <div class="import-pairing-entry">
                                            <span>{client.clientName}</span>
                                            <p>
                                                {client.scopes
                                                    .map(scope => importScopeLabels[scope])
                                                    .join(' · ')}
                                            </p>
                                            <Show when={client.rootIds.length}>
                                                <p>
                                                    {client.rootIds
                                                        .map(
                                                            id =>
                                                                current().roots.find(
                                                                    root => root.id === id
                                                                )?.name ?? 'Unavailable root'
                                                        )
                                                        .join(' · ')}
                                                </p>
                                            </Show>
                                            <div class="import-pairing-buttons">
                                                <Button
                                                    size="sm"
                                                    variant="outline"
                                                    disabled={disabled()}
                                                    onClick={() =>
                                                        void perform(
                                                            () =>
                                                                properties.pairing.revoke(
                                                                    current().catalogId,
                                                                    client.id
                                                                ),
                                                            'Application access revoked.',
                                                            'Could not revoke application access.'
                                                        )
                                                    }
                                                >
                                                    Revoke access
                                                </Button>
                                            </div>
                                        </div>
                                    )}
                                </For>
                            </div>
                        </>
                    )}
                </Show>
            </div>
        </SectionGroup>
    );
};
