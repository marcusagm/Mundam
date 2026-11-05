import { batch, Component, createMemo, createSignal, Show } from 'solid-js';
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';
import { Button, Checkbox, Modal } from '../../ui';
import { useFilters, useMetadata, useNotification } from '../../../core/hooks';
import { parseTagMarkdown, serializeTagMarkdown } from '../../../lib/tagMarkdown';
import './tag-bulk-modal.css';

/** Visibility contract for the Markdown taxonomy editor. */
interface TagBulkModalProperties {
    isOpen: boolean;
    onClose: () => void;
}

const EXAMPLE_OUTLINE = '- Art: color=#e57373\n  - Painting\n  - Drawing: color=#64b5f6\n- Design';

/** Imports ordered tag outlines and exports the current taxonomy as Markdown. */
export const TagBulkModal: Component<TagBulkModalProperties> = properties => {
    const metadata = useMetadata();
    const filters = useFilters();
    const notification = useNotification();
    const [markdown, setMarkdown] = createSignal('');
    const [keepExisting, setKeepExisting] = createSignal(true);
    const [isBusy, setIsBusy] = createSignal(false);
    const [error, setError] = createSignal('');
    const validation = createMemo(() => {
        try {
            return { count: parseTagMarkdown(markdown()).length, error: '' };
        } catch (error) {
            return { count: 0, error: String(error instanceof Error ? error.message : error) };
        }
    });

    /** Closes only after the active operation finishes. */
    const handleClose = (): void => {
        if (isBusy()) return;
        setKeepExisting(true);
        setError('');
        properties.onClose();
    };

    /** Saves the persisted taxonomy independently of the import editor. */
    const handleExport = async (): Promise<void> => {
        setIsBusy(true);
        setError('');
        try {
            const content = serializeTagMarkdown(metadata.tags);
            const destination = await save({
                defaultPath: 'mundam-tags.md',
                filters: [{ name: 'Markdown', extensions: ['md'] }]
            });
            if (!destination) return;
            await writeTextFile(destination, content);
            notification.success('Tags Exported');
        } catch (error) {
            setError(error instanceof Error ? error.message : 'Could not export tags.');
        } finally {
            setIsBusy(false);
        }
    };

    /** Commits the validated outline and refreshes dependent metadata. */
    const handleImport = async (): Promise<void> => {
        if (!validation().count || isBusy()) return;
        setIsBusy(true);
        setError('');
        try {
            await metadata.importTags(markdown(), !keepExisting());
            if (!keepExisting()) {
                batch(() => {
                    for (const tagId of [...filters.selectedTags]) filters.toggleTag(tagId);
                });
            }
            notification.success('Tags Imported', `${validation().count} tags processed.`);
            setMarkdown('');
            setKeepExisting(true);
            properties.onClose();
        } catch (error) {
            const message =
                error &&
                typeof error === 'object' &&
                'message' in error &&
                typeof error.message === 'string'
                    ? error.message
                    : 'Could not import tags. Check for conflicting existing names.';
            setError(message);
        } finally {
            setIsBusy(false);
        }
    };

    return (
        <Modal
            isOpen={properties.isOpen}
            onClose={handleClose}
            title="Import / Export Tags"
            size="lg"
            closeOnOverlayClick={false}
            footer={
                <>
                    <Button
                        variant="secondary"
                        disabled={isBusy() || !metadata.tags.length}
                        onClick={handleExport}
                    >
                        Export Existing Tags
                    </Button>
                    <Button variant="ghost" disabled={isBusy()} onClick={handleClose}>
                        Cancel
                    </Button>
                    <Button
                        variant={keepExisting() ? 'primary' : 'destructive'}
                        loading={isBusy()}
                        disabled={!validation().count}
                        onClick={handleImport}
                    >
                        {keepExisting() ? 'Import Tags' : 'Replace All Tags'}
                    </Button>
                </>
            }
        >
            <div class="tag-bulk-modal">
                <p>
                    Use two spaces per level. List order defines sibling order. Colors accept
                    hexadecimal values; omit the color or use color=inherit to follow the parent.
                </p>
                <label for="tag-markdown-outline">Markdown tag list</label>
                <textarea
                    id="tag-markdown-outline"
                    class="tag-bulk-outline"
                    value={markdown()}
                    placeholder={EXAMPLE_OUTLINE}
                    disabled={isBusy()}
                    spellcheck={false}
                    onInput={event => setMarkdown(event.currentTarget.value)}
                    aria-describedby="tag-outline-feedback"
                />
                <p id="tag-outline-feedback" aria-live="polite">
                    <Show when={validation().count} fallback={validation().error}>
                        {validation().count} tags ready to import.
                    </Show>
                </p>
                <Checkbox
                    label="Keep existing tags"
                    checked={keepExisting()}
                    disabled={isBusy()}
                    onCheckedChange={setKeepExisting}
                />
                <p>
                    Matching names under the same parent are updated and keep their asset
                    associations. Imported siblings appear after tags outside the list, in the
                    specified order.
                </p>
                <Show when={!keepExisting()}>
                    <p class="tag-bulk-warning" role="alert">
                        Replacing will delete ALL existing tags and remove ALL tag associations from
                        assets. This cannot be undone.
                    </p>
                </Show>
                <Show when={error()}>
                    <p class="tag-bulk-warning" role="alert">
                        {error()}
                    </p>
                </Show>
            </div>
        </Modal>
    );
};
