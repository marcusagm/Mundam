import { Component, Show } from 'solid-js';
import { ModalRoot } from './ModalRoot';
import { ModalOverlay } from './ModalOverlay';
import { ModalContent } from './ModalContent';
import { ModalHeader } from './ModalHeader';
import { ModalTitle } from './ModalTitle';
import { ModalCloseButton } from './ModalCloseButton';
import { ModalBody } from './ModalBody';
import { ModalFooter } from './ModalFooter';
import { ModalProperties } from './types';
import { detectPlatform } from '../../../core/input/utils/platform';
import './modal.css';

/**
 * Standard Modal component for dialogs and overlays.
 * Combines atomic modal components into a convenient, all-in-one component.
 *
 * @param componentProperties - Properties for the Modal.
 * @returns The rendered modal dialog.
 *
 * @example
 * <Modal isOpen={isOpen()} onClose={() => setIsOpen(false)} title="Settings">
 *   <p>Modal content goes here</p>
 * </Modal>
 */
export const Modal: Component<ModalProperties> = componentProperties => {
    const platform = detectPlatform();
    return (
        <ModalRoot isOpen={componentProperties.isOpen} onClose={componentProperties.onClose}>
            <ModalOverlay closeOnClick={componentProperties.closeOnOverlayClick} />
            <ModalContent
                size={componentProperties.size}
                class={componentProperties.class}
                role="dialog"
            >
                <Show
                    when={
                        componentProperties.title || componentProperties.showCloseButton !== false
                    }
                >
                    <ModalHeader platform={platform}>
                        <Show when={componentProperties.title}>
                            <ModalTitle>{componentProperties.title}</ModalTitle>
                        </Show>
                        <Show when={componentProperties.showCloseButton !== false}>
                            <ModalCloseButton />
                        </Show>
                    </ModalHeader>
                </Show>

                <ModalBody>{componentProperties.children}</ModalBody>

                <Show when={componentProperties.footer}>
                    <ModalFooter>{componentProperties.footer}</ModalFooter>
                </Show>
            </ModalContent>
        </ModalRoot>
    );
};
