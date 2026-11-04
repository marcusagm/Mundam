import { Component, createSignal, Show } from 'solid-js';
import { Modal } from '../../ui/Modal';
import { Button } from '../../ui/Button';
import { DuplicateRulesForm } from './DuplicateRulesForm';
import { useDuplicateRules } from './hooks/useDuplicateRules';
import './duplicate-rules-modal.css';
import { useDuplicateScan } from './hooks/useDuplicateScan';
import { useNotification } from '../../../core/hooks';

export interface DuplicateRulesModalProperties {
    /** Whether the duplicate rules configuration modal is open */
    isOpen: boolean;
    /** Callback invoked when the modal is closed */
    onClose: () => void;
}

/**
 * Modal dialog for inspecting and adjusting duplicate detection algorithm parameters.
 * Allows quick profile switching and triggering an immediate re-scan.
 *
 * @param {DuplicateRulesModalProperties} componentProperties - Reactive component properties.
 * @returns {JSX.Element} The rendered modal component.
 */
export const DuplicateRulesModal: Component<
    DuplicateRulesModalProperties
> = componentProperties => {
    const duplicateRules = useDuplicateRules();
    const scan = useDuplicateScan();
    const notification = useNotification();
    const [isSubmitting, setIsSubmitting] = createSignal(false);

    const handleSaveAndRescan = async (): Promise<void> => {
        if (isSubmitting() || !scan.canStart()) return;
        setIsSubmitting(true);
        try {
            if (await duplicateRules.saveAndRescan()) componentProperties.onClose();
        } catch (error) {
            console.error('Failed to save rules and trigger scan:', error);
            notification.error('Rules Not Saved', 'Could not save the detection rules. Try again.');
        } finally {
            setIsSubmitting(false);
        }
    };

    return (
        <Modal
            isOpen={componentProperties.isOpen}
            onClose={componentProperties.onClose}
            title="Duplicate Detection Rules"
            size="md"
            footer={
                <div class="duplicate-rules-modal-footer">
                    <Button
                        variant="ghost"
                        onClick={componentProperties.onClose}
                        disabled={duplicateRules.isSaving()}
                    >
                        Cancel
                    </Button>
                    <Button
                        variant="primary"
                        onClick={handleSaveAndRescan}
                        loading={duplicateRules.isSaving() || isSubmitting()}
                        disabled={!scan.canStart()}
                    >
                        Save and Rescan
                    </Button>
                </div>
            }
        >
            <Show when={scan.unavailableReason()}>
                <p role="status">{scan.unavailableReason()}</p>
            </Show>
            <DuplicateRulesForm
                selectedProfile={duplicateRules.selectedProfile()}
                considerExactMatch={duplicateRules.considerExactMatch()}
                considerVisualMatch={duplicateRules.considerVisualMatch()}
                considerCropMatch={duplicateRules.considerCropMatch()}
                ignoreResolutionDifference={duplicateRules.ignoreResolutionDifference()}
                ignoreRecompression={duplicateRules.ignoreRecompression()}
                allowRotation={duplicateRules.allowRotation()}
                allowMirroring={duplicateRules.allowMirroring()}
                minimumScore={duplicateRules.minimumScore()}
                minimumVisualScore={duplicateRules.minimumVisualScore()}
                minimumCropScore={duplicateRules.minimumCropScore()}
                isDisabled={duplicateRules.isSaving()}
                onProfileChange={duplicateRules.handleProfileChange}
                onConsiderExactMatchChange={duplicateRules.setConsiderExactMatch}
                onConsiderVisualMatchChange={duplicateRules.setConsiderVisualMatch}
                onConsiderCropMatchChange={duplicateRules.setConsiderCropMatch}
                onIgnoreResolutionDifferenceChange={duplicateRules.setIgnoreResolutionDifference}
                onIgnoreRecompressionChange={duplicateRules.setIgnoreRecompression}
                onAllowRotationChange={duplicateRules.setAllowRotation}
                onAllowMirroringChange={duplicateRules.setAllowMirroring}
                onMinimumScoreChange={duplicateRules.setMinimumScore}
                onMinimumVisualScoreChange={duplicateRules.setMinimumVisualScore}
                onMinimumCropScoreChange={duplicateRules.setMinimumCropScore}
            />
        </Modal>
    );
};
