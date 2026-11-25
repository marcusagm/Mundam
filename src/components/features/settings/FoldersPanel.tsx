import { createResource, createSignal, Show, type Component } from 'solid-js';
import { useMetadata } from '../../../core/hooks';
import type { ImportPreferenceClient } from '../../../lib/importPreferences';
import { Button } from '../../ui/Button';
import { SectionGroup } from '../../ui/SectionGroup';
import { FolderDestinationPicker } from '../library/FolderDestinationPicker';
import './folders-panel.css';

/** Displays and updates the catalog's default import folder. */
export const FoldersPanel: Component<{
    importPreferences: ImportPreferenceClient;
}> = properties => {
    const metadata = useMetadata();
    const [preference, { mutate, refetch }] = createResource(() =>
        properties.importPreferences.load().catch(() => null)
    );
    const [isSaving, setIsSaving] = createSignal(false);
    const [status, setStatus] = createSignal('');

    const save = async (folderId: string | null) => {
        const current = preference();
        if (!current || isSaving() || current.folderId === folderId) return;
        setIsSaving(true);
        setStatus('');
        try {
            mutate(await properties.importPreferences.save(current.catalogId, folderId));
            setStatus(folderId ? 'Default import folder saved.' : 'Default import folder cleared.');
        } catch {
            setStatus('Could not update the default import folder.');
        } finally {
            setIsSaving(false);
        }
    };

    return (
        <div class="settings-panel-content">
            <h2 class="settings-panel-title">Folders</h2>
            <p class="settings-panel-description">
                Choose where Mundam saves imports when a connected app leaves the destination blank.
            </p>
            <SectionGroup
                title="Default import folder"
                description="Connected apps can use this folder when they have permission to access it."
            >
                <div class="import-folder-settings">
                    <Show
                        when={!preference.loading}
                        fallback={<p role="status">Loading import settings…</p>}
                    >
                        <Show
                            when={preference()}
                            fallback={
                                <div class="import-folder-actions">
                                    <p role="alert">Could not load the default import folder.</p>
                                    <Button
                                        variant="outline"
                                        size="sm"
                                        onClick={() => void refetch()}
                                    >
                                        Retry
                                    </Button>
                                </div>
                            }
                        >
                            {current => (
                                <>
                                    <Show
                                        when={metadata.locations.length}
                                        fallback={
                                            <p>Add an indexed folder to choose a destination.</p>
                                        }
                                    >
                                        <FolderDestinationPicker
                                            value={current().folderId ?? ''}
                                            isDisabled={isSaving()}
                                            onSelect={folderId => void save(folderId)}
                                        />
                                    </Show>
                                    <div class="import-folder-actions">
                                        <p role="status">{status()}</p>
                                        <Button
                                            variant="outline"
                                            size="sm"
                                            disabled={isSaving() || !current().folderId}
                                            onClick={() => void save(null)}
                                        >
                                            Clear default
                                        </Button>
                                    </div>
                                </>
                            )}
                        </Show>
                    </Show>
                </div>
            </SectionGroup>
        </div>
    );
};
