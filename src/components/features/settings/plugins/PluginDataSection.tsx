import { createSignal, Show, type Component } from 'solid-js';
import type { PluginSettingsClient } from '../../../../lib/plugins';
import { Button } from '../../../ui/Button';
import { SectionGroup } from '../../../ui/SectionGroup';
import { PluginStoredDataControls } from './PluginStoredDataControls';

/** Exposes catalog data independently of the profile's installed package list. */
export const PluginDataSection: Component<
    Pick<PluginSettingsClient, 'exportAnnotations' | 'storedData'>
> = properties => {
    const [exporting, setExporting] = createSignal(false);
    const [count, setCount] = createSignal<number | null>(null);
    const [failed, setFailed] = createSignal(false);

    const exportAnnotations = async () => {
        if (exporting()) return;
        setExporting(true);
        setCount(null);
        setFailed(false);
        try {
            setCount(await properties.exportAnnotations());
        } catch {
            setFailed(true);
        } finally {
            setExporting(false);
        }
    };

    return (
        <SectionGroup
            title="Plugin data"
            description="Export saved annotations from this catalog, including those from removed plugins."
        >
            <div class="plugin-settings-section">
                <Button
                    size="sm"
                    variant="outline"
                    disabled={exporting()}
                    onClick={() => void exportAnnotations()}
                >
                    {exporting() ? 'Exporting…' : 'Export plugin annotations'}
                </Button>
                <Show when={failed()}>
                    <p role="alert">
                        Could not export plugin annotations. Choose a writable JSON file and try
                        again.
                    </p>
                </Show>
                <Show when={count() !== null}>
                    <p role="status">
                        Exported {count()} plugin {count() === 1 ? 'annotation' : 'annotations'}.
                    </p>
                </Show>
                <PluginStoredDataControls
                    client={properties.storedData}
                    exportInProgress={exporting()}
                />
            </div>
        </SectionGroup>
    );
};
