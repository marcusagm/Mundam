import { Component, Show } from 'solid-js';
import { Button } from '../Button';
import { ModalRoot } from './ModalRoot';
import { ModalOverlay } from './ModalOverlay';
import { ModalContent } from './ModalContent';
import { ModalHeader } from './ModalHeader';
import { ModalTitle } from './ModalTitle';
import { ModalCloseButton } from './ModalCloseButton';
import { ModalBody } from './ModalBody';
import { ModalFooter } from './ModalFooter';
import { ConfirmModalProperties } from './types';
import { detectPlatform } from '../../../core/input/utils/platform';

/**
 * A specialized Modal for confirmation dialogs.
 * Uses atomic modal components to provide a consistent confirmation experience.
 *
 * @param componentProperties - Properties for the ConfirmModal.
 * @returns The rendered confirmation modal.
 */
export const ConfirmModal: Component<ConfirmModalProperties> = componentProperties => {
    /**
     * Handles the confirmation action.
     * Invokes the callback and closes the modal.
     */
    const handleConfirm = () => {
        componentProperties.onConfirm();
        componentProperties.onClose();
    };

    /**
     * Resolves the confirm button variant based on the modal kind.
     */
    const resolvedConfirmVariant = () => {
        if (componentProperties.kind === 'danger') {
            return 'destructive';
        }
        return 'primary';
    };

    return (
        <ModalRoot isOpen={componentProperties.isOpen} onClose={componentProperties.onClose}>
            <ModalOverlay />
            <ModalContent size={componentProperties.size || 'sm'} role="alertdialog">
                <ModalHeader platform={detectPlatform()}>
                    <ModalTitle>{componentProperties.title}</ModalTitle>
                    <ModalCloseButton />
                </ModalHeader>

                <ModalBody>
                    <Show
                        when={componentProperties.children}
                        fallback={<p>{componentProperties.message}</p>}
                    >
                        {componentProperties.children}
                    </Show>
                </ModalBody>

                <ModalFooter>
                    <Button variant="secondary" onClick={componentProperties.onClose}>
                        {componentProperties.cancelText || 'Cancel'}
                    </Button>
                    <Button variant={resolvedConfirmVariant()} onClick={handleConfirm}>
                        {componentProperties.confirmText || 'Confirm'}
                    </Button>
                </ModalFooter>
            </ModalContent>
        </ModalRoot>
    );
};
