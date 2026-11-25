import { createSignal, For, Show, type Component } from 'solid-js';
import type { ImportApproval, ImportPairingStatus, ImportScope } from '../../../lib/externalImport';
import { Button } from '../../ui/Button';
import { Checkbox } from '../../ui/Checkbox';

/** Permission labels shared by the request and connected-client views. */
export const importScopeLabels: Record<ImportScope, string> = {
    'folders.read': 'Read selected folder trees',
    'tags.read': 'Read the catalog’s tags',
    'assets.import': 'Import into selected folder trees',
    'tags.assignOnImport': 'Assign existing tags to imported files'
};

interface ImportConsentRequestProperties {
    request: ImportPairingStatus['pending'][number];
    catalog: ImportPairingStatus;
    disabled: boolean;
    approve(approval: ImportApproval): void;
    reject(requestId: string): void;
}

/** Lets the user choose the smallest grant appropriate for a pending client. */
export const ImportConsentRequest: Component<ImportConsentRequestProperties> = properties => {
    const [scopes, setScopes] = createSignal<ImportScope[]>([]);
    const [rootIds, setRootIds] = createSignal<string[]>([]);
    const needsRoots = () =>
        scopes().includes('folders.read') || scopes().includes('assets.import');
    const selectedRoots = () =>
        properties.catalog.roots.filter(root => rootIds().includes(root.id));
    const canApprove = () =>
        !properties.disabled &&
        scopes().length > 0 &&
        (!needsRoots() || selectedRoots().length > 0);

    const selectScope = (scope: ImportScope, checked: boolean) => {
        setScopes(current =>
            checked
                ? [...current, scope]
                : current.filter(
                      candidate =>
                          candidate !== scope &&
                          !(scope === 'assets.import' && candidate === 'tags.assignOnImport')
                  )
        );
    };

    return (
        <div class="import-pairing-entry">
            <h4>{properties.request.clientName}</h4>
            <fieldset>
                <legend>Requested permissions</legend>
                <For each={properties.request.requestedScopes}>
                    {scope => (
                        <Checkbox
                            size="sm"
                            label={importScopeLabels[scope]}
                            checked={scopes().includes(scope)}
                            disabled={
                                properties.disabled ||
                                (scope === 'tags.assignOnImport' &&
                                    !scopes().includes('assets.import'))
                            }
                            onCheckedChange={checked => selectScope(scope, checked)}
                        />
                    )}
                </For>
            </fieldset>
            <Show when={needsRoots()}>
                <fieldset>
                    <legend>Allowed library roots</legend>
                    <For
                        each={properties.catalog.roots}
                        fallback={<p>Add an indexed root to grant folder access.</p>}
                    >
                        {root => (
                            <Checkbox
                                size="sm"
                                label={root.name}
                                description={root.path}
                                checked={rootIds().includes(root.id)}
                                disabled={properties.disabled}
                                onCheckedChange={checked =>
                                    setRootIds(current =>
                                        checked
                                            ? [...current, root.id]
                                            : current.filter(id => id !== root.id)
                                    )
                                }
                            />
                        )}
                    </For>
                </fieldset>
            </Show>
            <div class="import-pairing-buttons">
                <Button
                    size="sm"
                    variant="outline"
                    disabled={properties.disabled}
                    onClick={() => properties.reject(properties.request.id)}
                >
                    Decline
                </Button>
                <Button
                    size="sm"
                    disabled={!canApprove()}
                    onClick={() =>
                        properties.approve({
                            requestId: properties.request.id,
                            catalogId: properties.catalog.catalogId,
                            scopes: scopes(),
                            rootIds: needsRoots() ? selectedRoots().map(root => root.id) : []
                        })
                    }
                >
                    Approve
                </Button>
            </div>
        </div>
    );
};
