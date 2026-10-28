import { Component, JSX } from 'solid-js';
import { cn } from '../../../lib/utils';

/**
 * Properties for the ModalHeader component.
 */
interface ModalHeaderProperties {
    /** The content of the header. */
    children: JSX.Element;
    /** Additional CSS classes. */
    class?: string;
    /**
     * Platform hint used to apply OS-specific close-button placement.
     * When 'mac', the close button is placed on the left via CSS flex ordering.
     * Defaults to Windows/Linux (close on the right).
     */
    platform?: 'mac' | 'windows' | 'linux';
}

/**
 * Layout component for the modal header.
 *
 * @param componentProperties - Properties for the ModalHeader.
 * @returns The rendered header element.
 */
export const ModalHeader: Component<ModalHeaderProperties> = componentProperties => {
    return (
        <header
            class={cn(
                'ui-modal-header',
                componentProperties.platform === 'mac' && 'ui-modal-header--macos',
                componentProperties.class
            )}
        >
            {componentProperties.children}
        </header>
    );
};
