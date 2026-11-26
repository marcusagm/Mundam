import {
    createEffect,
    createMemo,
    createResource,
    createSignal,
    onCleanup,
    type Accessor
} from 'solid-js';
import type {
    PluginInspectorClient,
    PluginInspectorSection
} from '../../../../lib/pluginInspector';

/** Shared gallery projection. Undefined is inactive or loading. Null indicates a failed query. */
export interface PluginInspectorPresentation {
    sections: Accessor<PluginInspectorSection[] | null | undefined>;
    refresh: () => void;
}

/** Loads once for both gallery surfaces and hides earlier results during an invalidation. */
export function createPluginInspectorPresentation(
    client: Accessor<PluginInspectorClient>,
    eligible: Accessor<boolean>
): PluginInspectorPresentation {
    const isEligible = createMemo(eligible);
    const [revision, setRevision] = createSignal(0);
    const refresh = () => setRevision(current => current + 1);
    const [sections] = createResource(
        () => isEligible() && { client: client(), revision: revision() },
        ({ client }) => client.load().catch(() => null)
    );
    createEffect(() => {
        if (isEligible()) onCleanup(client().subscribe(refresh));
    });

    return {
        sections: () => (isEligible() && !sections.loading ? sections() : undefined),
        refresh
    };
}
