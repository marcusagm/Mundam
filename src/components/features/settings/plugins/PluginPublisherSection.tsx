import { createSignal, For, Show, type Component } from 'solid-js';
import type { PluginPublisher } from '../../../../lib/plugins';
import { Button } from '../../../ui/Button';
import { Checkbox } from '../../../ui/Checkbox';
import { Input } from '../../../ui/Input';
import { SectionGroup } from '../../../ui/SectionGroup';

/** Collects trust separately from package selection and confirms permanent revocation. */
export const PluginPublisherSection: Component<{
    publishers: PluginPublisher[];
    busy: boolean;
    onTrust(namespace: string, fingerprint: string): Promise<boolean>;
    onRevoke(publisher: PluginPublisher): Promise<boolean>;
}> = properties => {
    const [namespace, setNamespace] = createSignal('');
    const [fingerprint, setFingerprint] = createSignal('');
    const [verified, setVerified] = createSignal(false);
    const [revocation, setRevocation] = createSignal<string>();
    const validIdentity = () => namespace().trim() && /^[0-9a-f]{64}$/i.test(fingerprint().trim());

    const submit = async (event: SubmitEvent) => {
        event.preventDefault();
        if (properties.busy || !verified() || !validIdentity()) return;
        if (await properties.onTrust(namespace().trim(), fingerprint().trim())) {
            setNamespace('');
            setFingerprint('');
            setVerified(false);
        }
    };

    const revoke = async (publisher: PluginPublisher) => {
        if (await properties.onRevoke(publisher)) setRevocation(undefined);
    };

    return (
        <SectionGroup
            title="Trusted publishers"
            description="Get the namespace and SHA-256 fingerprint from a trusted source before choosing a package."
        >
            <div class="plugin-settings-section">
                <form class="plugin-settings-fields" onSubmit={event => void submit(event)}>
                    <Input
                        size="sm"
                        label="Publisher namespace"
                        aria-label="Publisher namespace"
                        autocomplete="off"
                        spellcheck={false}
                        maxLength={128}
                        placeholder="org.example"
                        value={namespace()}
                        disabled={properties.busy}
                        onInput={event => {
                            setNamespace(event.currentTarget.value);
                            setVerified(false);
                        }}
                    />
                    <Input
                        size="sm"
                        label="SHA-256 fingerprint"
                        aria-label="SHA-256 fingerprint"
                        autocomplete="off"
                        spellcheck={false}
                        value={fingerprint()}
                        disabled={properties.busy}
                        onInput={event => {
                            setFingerprint(event.currentTarget.value);
                            setVerified(false);
                        }}
                    />
                    <Checkbox
                        size="sm"
                        label="I verified this publisher identity independently"
                        checked={verified()}
                        disabled={properties.busy}
                        onCheckedChange={setVerified}
                    />
                    <div class="plugin-settings-actions">
                        <Button
                            type="submit"
                            size="sm"
                            variant="outline"
                            disabled={properties.busy || !validIdentity() || !verified()}
                        >
                            Trust publisher
                        </Button>
                    </div>
                </form>
                <Show
                    when={properties.publishers.length}
                    fallback={<p>No publishers registered.</p>}
                >
                    <ul class="plugin-settings-list" aria-label="Registered publishers">
                        <For each={properties.publishers}>
                            {publisher => (
                                <li class="plugin-settings-entry">
                                    <div class="plugin-settings-actions">
                                        <strong>{publisher.namespace}</strong>
                                        <span>{publisher.revoked ? 'Revoked' : 'Trusted'}</span>
                                    </div>
                                    <code>{publisher.fingerprint}</code>
                                    <Show when={!publisher.revoked}>
                                        <Show
                                            when={revocation() === publisher.namespace}
                                            fallback={
                                                <div class="plugin-settings-actions">
                                                    <Button
                                                        size="sm"
                                                        variant="ghost-destructive"
                                                        disabled={properties.busy}
                                                        onClick={() =>
                                                            setRevocation(publisher.namespace)
                                                        }
                                                    >
                                                        Revoke trust
                                                    </Button>
                                                </div>
                                            }
                                        >
                                            <p role="status">
                                                Revoking this key is permanent and disables its
                                                plugins.
                                            </p>
                                            <div class="plugin-settings-actions">
                                                <Button
                                                    size="sm"
                                                    variant="destructive"
                                                    disabled={properties.busy}
                                                    onClick={() => void revoke(publisher)}
                                                >
                                                    Confirm revocation
                                                </Button>
                                                <Button
                                                    size="sm"
                                                    variant="outline"
                                                    disabled={properties.busy}
                                                    onClick={() => setRevocation(undefined)}
                                                >
                                                    Cancel
                                                </Button>
                                            </div>
                                        </Show>
                                    </Show>
                                </li>
                            )}
                        </For>
                    </ul>
                </Show>
            </div>
        </SectionGroup>
    );
};
