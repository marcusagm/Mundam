import { Component, untrack } from 'solid-js';
import { ModalRoot, ModalOverlay } from '../../ui/Modal';
import { DuplicateCandidate } from './types';
import { createConditionalScope, useShortcuts } from '../../../core/input';
import { MetadataMergeModalInner } from './MetadataMergeModalInner';

export interface MetadataMergeModalProperties {
    /** Whether the modal is currently visible. */
    isOpen: boolean;
    /** The ID of the group being resolved. */
    groupId: string;
    /** All candidates in the group. */
    candidates: DuplicateCandidate[];
    /** The candidates the user chose to keep. */
    keptCandidates: DuplicateCandidate[];
    /** The candidates that will be discarded. */
    discardedCandidates: DuplicateCandidate[];
    /** Callback to resolve the group. Provided by the parent's resolveGroup function. */
    onResolve: (groupId: string, action: string, keptAssetIds?: string[]) => Promise<void>;
    /** Callback to close the modal without applying. */
    onClose: () => void;
}

/**
 * A modal that allows the user to review and confirm metadata merge decisions
 * field-by-field before resolving a duplicate group.
 *
 * Collects: rating, favorite status, notes, tags from discarded candidates,
 * and divergent EXIF/XMP/IPTC technical fields.
 */
export const MetadataMergeModal: Component<MetadataMergeModalProperties> = props => {
    // Untrack groupId because it is only used once to define the scope ID,
    // avoiding the SolidJS linter warning for props used outside tracked scope.
    const scopeId = `merge-modal-${untrack(() => props.groupId)}`;

    createConditionalScope(scopeId, () => props.isOpen, 1200, true);

    useShortcuts([
        {
            keys: 'Escape',
            scope: scopeId,
            system: true,
            enabled: () => props.isOpen,
            action: () => props.onClose()
        }
    ]);

    return (
        <ModalRoot isOpen={props.isOpen} onClose={props.onClose}>
            <ModalOverlay closeOnClick={false} />
            <MetadataMergeModalInner {...props} />
        </ModalRoot>
    );
};
