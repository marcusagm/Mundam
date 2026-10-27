import { ConfirmModal } from '../../ui';
import { Component, createSignal } from 'solid-js';
import { useNotification, useMetadata } from '../../../core/hooks';
import { invokeCommand } from '../../../lib/api';
import { libraryActions } from '../../../core/store/library';
import './trash-empty-modal.css';

/**
 * Properties for the TrashEmptyModal component.
 */
interface TrashEmptyModalProperties {
    /** Whether the modal is open. */
    isOpen: boolean;
    /** Callback to close the modal. */
    onClose: () => void;
    /** Number of items currently in the trash. */
    trashCount: number;
}

/**
 * Modal to confirm permanently emptying the trash.
 *
 * Displays the number of items that will be permanently deleted
 * and warns the user that this action cannot be undone.
 *
 * @param componentProperties - Properties for the component.
 * @returns The rendered TrashEmptyModal.
 */
export const TrashEmptyModal: Component<TrashEmptyModalProperties> = componentProperties => {
    const notification = useNotification();
    const metadata = useMetadata();
    const [isEmptying, setIsEmptying] = createSignal(false);

    /**
     * Handles the confirmation of emptying the trash.
     * Invokes the backend command and refreshes the library state.
     */
    const handleConfirm = async () => {
        setIsEmptying(true);
        try {
            const deletedCount = await invokeCommand<number>('empty_trash');
            notification.success(
                'Trash Emptied',
                `Permanently deleted ${deletedCount} item${deletedCount !== 1 ? 's' : ''}.`
            );
            await libraryActions.refreshAssets(true);
            await metadata.loadStats();
        } catch (error) {
            notification.error('Failed to empty trash.', String(error));
            console.error('TrashEmptyModal: empty_trash failed:', error);
        } finally {
            setIsEmptying(false);
            componentProperties.onClose();
        }
    };

    return (
        <ConfirmModal
            isOpen={componentProperties.isOpen}
            onClose={componentProperties.onClose}
            onConfirm={handleConfirm}
            title="Empty Trash"
            kind="danger"
            confirmText={isEmptying() ? 'Deleting...' : 'Delete Permanently'}
            message=""
        >
            <div class="trash-empty-modal-content">
                <p>
                    Are you sure you want to permanently delete{' '}
                    <strong>
                        {componentProperties.trashCount} item
                        {componentProperties.trashCount !== 1 ? 's' : ''}
                    </strong>{' '}
                    from the trash?
                </p>
                <p class="trash-empty-warning">
                    This action cannot be undone. All files will be permanently removed lost.
                </p>
            </div>
        </ConfirmModal>
    );
};
