import { ConfirmModal } from '../../ui';
import { Component, Show } from 'solid-js';
import { TreeNode } from '../../ui/TreeView';
import { useMetadata, useNotification } from '../../../core/hooks';
import './tag-delete-modal.css';

/**
 * Properties for the TagDeleteModal component.
 */
interface TagDeleteModalProperties {
    /** Whether the modal is open. */
    isOpen: boolean;
    /** Callback to close the modal. */
    onClose: () => void;
    /** The tag node to be deleted. */
    node: TreeNode | null;
}

/**
 * Modal to confirm the deletion of a tag and its descendants.
 *
 * @param componentProperties - Properties for the component.
 * @returns The rendered TagDeleteModal.
 */
export const TagDeleteModal: Component<TagDeleteModalProperties> = componentProperties => {
    const metadata = useMetadata();
    const { deleteTag, createTag } = metadata;
    const notification = useNotification();

    /**
     * Recursively retrieves all descendant identifiers of a given node.
     *
     * @param node - The starting tree node.
     * @returns An array of descendant numeric identifiers.
     */
    const getAllDescendantIdentifiers = (node: TreeNode): string[] => {
        let identifiers: string[] = [];
        if (node.children) {
            node.children.forEach(child => {
                identifiers.push(String(child.id));
                identifiers = [...identifiers, ...getAllDescendantIdentifiers(child)];
            });
        }
        return identifiers;
    };

    /**
     * Handles the confirmation of tag deletion.
     */
    const handleConfirm = async () => {
        const node = componentProperties.node;
        if (!node) {
            return;
        }

        const tagName = node.label;
        const tagIdentifier = String(node.id);
        const currentTag = metadata.tags.find(tag => tag.id === tagIdentifier);
        const parentIdentifier = currentTag?.parent_id;
        const tagColor = currentTag?.inherits_color ? undefined : currentTag?.color;

        const result = await deleteTag(tagIdentifier);

        if (result.success) {
            notification.success('Tag Deleted', `Removed "${tagName}"`, {
                label: 'Undo',
                onClick: async () => {
                    const restoreResult = await createTag(tagName, parentIdentifier, tagColor);
                    if (restoreResult.success) {
                        notification.success('Restore Successful', `Tag "${tagName}" restored`);
                    } else {
                        notification.error('Restore Failed');
                    }
                }
            });
            componentProperties.onClose();
        } else {
            notification.error(result.error?.message || 'Failed to Delete Tag');
        }
    };

    /**
     * Calculates the number of descendant tags.
     */
    const descendantCount = () =>
        componentProperties.node ? getAllDescendantIdentifiers(componentProperties.node).length : 0;

    return (
        <ConfirmModal
            isOpen={componentProperties.isOpen}
            onClose={componentProperties.onClose}
            onConfirm={handleConfirm}
            title="Delete Tag"
            kind="danger"
            confirmText="Delete"
            message=""
        >
            <div class="tag-delete-modal-content">
                <p>
                    Are you sure you want to delete tag{' '}
                    <strong>"{componentProperties.node?.label}"</strong>?
                </p>
                <Show when={descendantCount() > 0}>
                    <p class="tag-delete-warning">
                        This will also delete <strong>{descendantCount()}</strong> child tags. This
                        action cannot be undone.
                    </p>
                </Show>
            </div>
        </ConfirmModal>
    );
};
