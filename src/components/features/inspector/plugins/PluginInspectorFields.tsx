import {
    createResource,
    createSignal,
    For,
    Match,
    onCleanup,
    Show,
    Switch,
    type Component
} from 'solid-js';
import type {
    PluginInspectorDataClient,
    PluginInspectorSection,
    PluginSectionTarget
} from '../../../../lib/pluginInspector';
import { Button } from '../../../ui/Button';
import { Checkbox } from '../../../ui/Checkbox';

/** Edits confirmed field values for one asset and one approved package revision. */
export const PluginInspectorFields: Component<{
    section: PluginInspectorSection;
    assetId: string;
    client: PluginInspectorDataClient;
}> = properties => {
    const target = (): PluginSectionTarget => ({
        assetId: properties.assetId,
        pluginId: properties.section.pluginId,
        sectionId: properties.section.sectionId,
        packageRevision: properties.section.packageRevision
    });
    const [values, { mutate, refetch }] = createResource(
        () => ({ target: target(), client: properties.client }),
        request => request.client.load(request.target).catch(() => null)
    );
    const [saving, setSaving] = createSignal(false);
    const [feedback, setFeedback] = createSignal('');
    let active = true;
    onCleanup(() => {
        active = false;
    });

    const writableField = (fieldId: string) => {
        if (values.loading || saving()) return;
        const current = values()?.find(field => field.fieldId === fieldId)?.state;
        return current?.status === 'available' && current.writable ? current : undefined;
    };

    const change = async (fieldId: string, value: boolean) => {
        const current = writableField(fieldId);
        if (!current) return;
        setSaving(true);
        setFeedback('');
        try {
            const result = await properties.client.save({
                target: target(),
                fieldId,
                expectedRevision: current.revision,
                value
            });
            if (!active) return;
            if (result.status === 'saved') {
                mutate(values =>
                    values?.map(field =>
                        field.fieldId === fieldId
                            ? {
                                  fieldId,
                                  state: {
                                      ...current,
                                      value: result.value,
                                      revision: result.revision
                                  }
                              }
                            : field
                    )
                );
            } else {
                setFeedback(
                    result.status === 'conflict'
                        ? 'This value changed elsewhere. The latest value was requested.'
                        : 'This field requires a plugin data update.'
                );
                await refetch();
            }
        } catch {
            if (!active) return;
            setFeedback('Could not save the plugin field.');
            await refetch();
        } finally {
            if (active) setSaving(false);
        }
    };

    return (
        <div class="plugin-inspector-fields">
            <p>{properties.section.pluginName}</p>
            <Show when={feedback()}>
                <p role="status">{feedback()}</p>
            </Show>
            <Switch>
                <Match when={values.loading}>
                    <p role="status">Loading plugin values…</p>
                </Match>
                <Match when={values() === null}>
                    <p role="status">Could not load plugin values.</p>
                    <Button
                        size="xs"
                        variant="outline"
                        onClick={() => {
                            setFeedback('');
                            void refetch();
                        }}
                    >
                        Retry
                    </Button>
                </Match>
                <Match when={values()}>
                    {fields => (
                        <For each={properties.section.fields}>
                            {field => {
                                const state = () =>
                                    fields().find(item => item.fieldId === field.fieldId)?.state;
                                const available = () => {
                                    const current = state();
                                    return current?.status === 'available' ? current : undefined;
                                };
                                return (
                                    <Show
                                        when={available()}
                                        fallback={
                                            <p>
                                                {field.label} ·{' '}
                                                {state()
                                                    ? 'Requires a plugin data update'
                                                    : 'Unavailable'}
                                            </p>
                                        }
                                    >
                                        {current => (
                                            <Checkbox
                                                size="sm"
                                                disabled={!current().writable || saving()}
                                                label={field.label}
                                                checked={current().value}
                                                onCheckedChange={value =>
                                                    void change(field.fieldId, value)
                                                }
                                            />
                                        )}
                                    </Show>
                                );
                            }}
                        </For>
                    )}
                </Match>
            </Switch>
        </div>
    );
};
