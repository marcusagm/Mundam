import { ConfirmModal } from '../../ui';
import { Component } from 'solid-js';
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
    const { removeLocation } = useLibrary();
    const notification = useNotification();

    /**
     * Handles the folder removal confirmation.
     * Invokes the backend to stop monitoring the location.
     */
    const handleConfirm = () => {
        if (componentProperties.folderIdentifier === null) {
            return;
        }

        removeLocation(componentProperties.folderIdentifier).then(result => {
            if (result.success) {
                notification.success(
                    'Folder Removed',
                    `Stopped monitoring "${componentProperties.folderName}"`
                );
            } else {
                notification.error('Failed to Remove Folder');
            }
            componentProperties.onClose();
        });
    };

    return (
        <ConfirmModal
            isOpen={componentProperties.isOpen}
            onClose={componentProperties.onClose}
            onConfirm={handleConfirm}
            title="Remove Folder"
            kind="danger"
            confirmText="Remove"
            message=""
        >
            <div class="folder-delete-modal-content">
                <p>
                    Are you sure you want to remove{' '}
                    <strong>"{componentProperties.folderName}"</strong> from the library?
                </p>
                <p class="folder-delete-warning">
                    This will remove this folder, its subfolders, and all their assets from the
                    library, including generated thumbnails and previews. The original files will{' '}
                    <strong>not</strong> be deleted.
                </p>
            </div>
        </ConfirmModal>
    );
};
