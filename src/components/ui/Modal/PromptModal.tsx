import { Component, Show, createSignal, createEffect, createMemo } from 'solid-js';
import { ModalRoot } from './ModalRoot';
import { ModalOverlay } from './ModalOverlay';
import { ModalContent } from './ModalContent';
import { ModalHeader } from './ModalHeader';
import { ModalTitle } from './ModalTitle';
import { ModalCloseButton } from './ModalCloseButton';
import { ModalBody } from './ModalBody';
import { ModalFooter } from './ModalFooter';
import { Button } from '../Button';
import { Input } from '../Input';
import { PromptModalProperties } from './types';
import { createId } from '../../../lib/primitives/createId';
import { detectPlatform } from '../../../core/input/utils/platform';

/**
 * A specialized Modal component for capturing user input text with validation.
 * Uses a form element to provide native submission on Enter key press.
 *
 * @param componentProperties - Properties for the PromptModal.
 * @returns The rendered prompt modal dialog.
 *
 * @example
 * <PromptModal
 *   isOpen={isOpen()}
 *   onClose={() => setIsOpen(false)}
 *   onConfirm={(value) => console.log(value)}
 *   title="Rename Folder"
 *   placeholder="Enter new name..."
 * />
 */
export const PromptModal: Component<PromptModalProperties> = componentProperties => {
    let inputReference: HTMLInputElement | undefined;

    const [inputValue, setInputValue] = createSignal(componentProperties.initialValue || '');
    const [internalErrorMessage, setInternalErrorMessage] = createSignal<string | null>(null);

    // Synchronize the input value and clear errors when the modal opens.
    createEffect(() => {
        if (componentProperties.isOpen) {
            setInputValue(componentProperties.initialValue || '');
            setInternalErrorMessage(null);
        }
    });

    /**
     * Resolves the active error message, prioritizing external errors.
     */
    const activeErrorMessage = createMemo(
        () => componentProperties.errorMessage || internalErrorMessage()
    );

    const descriptionIdentifier = createId('prompt-modal-description');

    // Focus the input field automatically when the modal opens.
    createEffect(() => {
        if (componentProperties.isOpen && inputReference) {
            // A short delay ensures the modal and portal are fully mounted.
            setTimeout(() => inputReference?.focus(), 50);
        }
    });

    /**
     * Handles the form submission event.
     * Performs validation before invoking the confirmation callback.
     *
     * @param event - The submission event.
     */
    const handleFormSubmit = (event: SubmitEvent) => {
        event.preventDefault();

        // Check for required field validation.
        if (componentProperties.required && !inputValue().trim()) {
            setInternalErrorMessage('This field is required.');
            return;
        }

        // Run custom validation function if provided.
        if (componentProperties.validate) {
            const validationResult = componentProperties.validate(inputValue());
            if (validationResult) {
                setInternalErrorMessage(validationResult);
                return;
            }
        }

        componentProperties.onConfirm(inputValue());
        componentProperties.onClose();
    };

    return (
        <ModalRoot isOpen={componentProperties.isOpen} onClose={componentProperties.onClose}>
            <ModalOverlay />
            <ModalContent size="sm">
                <form onSubmit={handleFormSubmit} novalidate>
                    <ModalHeader platform={detectPlatform()}>
                        <ModalTitle>{componentProperties.title}</ModalTitle>
                        <ModalCloseButton />
                    </ModalHeader>

                    <ModalBody>
                        <Show when={componentProperties.description}>
                            <p id={descriptionIdentifier} class="ui-modal-description">
                                {componentProperties.description}
                            </p>
                        </Show>

                        <Input
                            ref={inputReference}
                            value={inputValue()}
                            onInput={event => {
                                setInputValue(event.currentTarget.value);
                                if (internalErrorMessage()) {
                                    setInternalErrorMessage(null);
                                }
                            }}
                            placeholder={componentProperties.placeholder}
                            error={!!activeErrorMessage()}
                            errorMessage={activeErrorMessage() || undefined}
                            required={componentProperties.required}
                            aria-describedby={
                                componentProperties.description ? descriptionIdentifier : undefined
                            }
                        />
                    </ModalBody>

                    <ModalFooter>
                        <Button type="button" variant="ghost" onClick={componentProperties.onClose}>
                            {componentProperties.cancelText || 'Cancel'}
                        </Button>
                        <Button type="submit" variant="primary">
                            {componentProperties.confirmText || 'Confirm'}
                        </Button>
                    </ModalFooter>
                </form>
            </ModalContent>
        </ModalRoot>
    );
};
