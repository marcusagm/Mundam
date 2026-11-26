import {
    createEffect,
    createMemo,
    createResource,
    createSignal,
    onCleanup,
    type Accessor
} from 'solid-js';
import type { PluginInspectorSection } from '../../lib/pluginInspector';
import type { PluginPresentation, PluginPresentationClient } from '../../lib/pluginPresentation';

/** Shared application projection. Undefined is inactive or loading. Null indicates failure. */
export interface PluginPresentationState {
    snapshot: Accessor<PluginPresentation | null | undefined>;
    sections: Accessor<PluginInspectorSection[] | null | undefined>;
    refresh: () => void;
}

/** Shares definitions across the application and reloads them when catalog identity changes. */
export function createPluginPresentation(
    client: Accessor<PluginPresentationClient>,
    catalog: Accessor<string | undefined>
): PluginPresentationState {
    const scope = createMemo(catalog);
    const [revision, setRevision] = createSignal(0);
    const refresh = () => setRevision(current => current + 1);
    const [resource] = createResource(
        () => (scope() ? { client: client(), revision: revision() } : false),
        ({ client }) => client.load().catch(() => null)
    );
    createEffect(() => {
        if (scope()) onCleanup(client().subscribe(refresh));
    });

    const snapshot = () => (scope() && !resource.loading ? resource() : undefined);
    return {
        snapshot,
        sections: () => {
            const current = snapshot();
            return current ? current.sections : current;
        },
        refresh
    };
}
