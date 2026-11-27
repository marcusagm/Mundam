import { Button, Modal } from '../../ui';
import { createEffect, createResource, createSignal, Show, type Component } from 'solid-js';
import { useLibrary, useNotification } from '../../../core/hooks';
import './folder-delete-modal.css';

/**
 * Properties for the FolderDeleteModal component.
 */
interface FolderDeleteModalProperties {
    /** Whether the modal is visible. */
    isOpen: boolean;
    /** Callback invoked when the modal requests closure. */
    onClose: () => void;
    /** The unique identifier of the folder to be removed. */
    folderIdentifier: string | null;
    /** The display name of the folder. */
    folderName: string;
}

/**
 * Modal dialog to confirm the removal/unmonitoring of a folder in the library.
 *
 * @param componentProperties - Properties for the component.
 * @returns The rendered FolderDeleteModal.
 */
export const FolderDeleteModal: Component<FolderDeleteModalProperties> = componentProperties => {
    const { removeLocation, reviewLocationRemoval } = useLibrary();
    const notification = useNotification();
    const [impact, { refetch }] = createResource(
        () => componentProperties.isOpen && componentProperties.folderIdentifier,
        folderId => reviewLocationRemoval(folderId).catch(() => null)
    );
    const [busy, setBusy] = createSignal(false);
    const [failed, setFailed] = createSignal(false);
    const reviewed = () => (impact.state === 'ready' ? impact() : undefined);
    const close = () => {
        if (!busy()) componentProperties.onClose();
    };

    createEffect(() => {
        if (!componentProperties.isOpen) setFailed(false);
    });

    const handleConfirm = async () => {
        const folderId = componentProperties.folderIdentifier;
        const current = reviewed();
        if (busy() || !folderId || !current) return;
        setBusy(true);
        setFailed(false);
        const result = await removeLocation(folderId, current.annotationRevision);
        setBusy(false);
        if (result.success) {
            notification.success(
                'Folder Unlinked',
                `Stopped monitoring "${componentProperties.folderName}"`
            );
            close();
        } else {
            setFailed(true);
            void refetch();
        }
    };

    return (
        <Modal
            isOpen={componentProperties.isOpen}
            onClose={close}
            title="Unlink Folder"
            size="sm"
            footer={
                <>
                    <Button variant="secondary" disabled={busy()} onClick={close}>
                        Cancel
                    </Button>
                    <Button
                        variant="destructive"
                        disabled={busy() || !reviewed()}
                        onClick={() => void handleConfirm()}
                    >
                        Unlink Folder
                    </Button>
                </>
            }
        >
            <div class="folder-delete-modal-content">
                <p>
                    Unlink <strong>"{componentProperties.folderName}"</strong> from the library?
                </p>
                <p class="folder-delete-warning">
                    The folder and its assets will be removed from the catalog. The original files
                    will remain on disk.
                </p>
                <Show when={impact.loading}>
                    <p role="status">Checking saved plugin annotations…</p>
                </Show>
                <Show when={reviewed() === null}>
                    <p role="alert">Could not check saved plugin annotations.</p>
                    <Button variant="outline" onClick={() => void refetch()}>
                        Retry
                    </Button>
                </Show>
                <Show when={reviewed()?.annotationCount}>
                    {count => (
                        <p class="folder-delete-warning">
                            Unlinking permanently removes {count()} saved plugin{' '}
                            {count() === 1 ? 'annotation' : 'annotations'}. To keep a copy, export
                            plugin annotations from Settings before unlinking.
                        </p>
                    )}
                </Show>
                <Show when={failed()}>
                    <p role="alert">Could not unlink. Review the current impact and try again.</p>
                </Show>
            </div>
        </Modal>
    );
};
