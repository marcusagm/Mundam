import { Component, createSignal } from 'solid-js';
import { Modal } from '../../ui/Modal';
import { Button } from '../../ui/Button';
import { Select } from '../../ui/Select';
import { Slider } from '../../ui/Slider';
import { Switch } from '../../ui/Switch';
import { Info } from 'lucide-solid';
import { duplicatesApi } from '../../../lib/duplicates';
import './duplicate-rules-modal.css';

export interface DuplicateRulesModalProperties {
    isOpen: boolean;
    onClose: () => void;
}

export const DuplicateRulesModal: Component<DuplicateRulesModalProperties> = props => {
    const [selectedProfile, setSelectedProfile] = createSignal('visual-match');

    const [considerExactMatch, setConsiderExactMatch] = createSignal(true);
    const [considerVisualMatch, setConsiderVisualMatch] = createSignal(true);
    const [considerCropMatch, setConsiderCropMatch] = createSignal(false);
    const [ignoreResolutionDifference, setIgnoreResolutionDifference] = createSignal(true);
    const [ignoreRecompression, setIgnoreRecompression] = createSignal(true);
    const [minScore, setMinScore] = createSignal(0.85);

    const profiles = [
        { value: 'exact-match', label: 'Strict Exact Match' },
        { value: 'visual-match', label: 'Visual Similarity (Default)' },
        { value: 'aggressive', label: 'Aggressive deduplication' },
        { value: 'custom', label: 'Custom rules' }
    ];

    const handleProfileChange = (value: string) => {
        setSelectedProfile(value);
        if (value === 'exact-match') {
            setConsiderExactMatch(true);
            setConsiderVisualMatch(false);
            setConsiderCropMatch(false);
            setIgnoreResolutionDifference(false);
            setIgnoreRecompression(false);
            setMinScore(1.0);
        } else if (value === 'visual-match') {
            setConsiderExactMatch(false);
            setConsiderVisualMatch(true);
            setConsiderCropMatch(false);
            setIgnoreResolutionDifference(true);
            setIgnoreRecompression(true);
            setMinScore(0.9);
        } else if (value === 'aggressive') {
            setConsiderExactMatch(true);
            setConsiderVisualMatch(true);
            setConsiderCropMatch(true);
            setIgnoreResolutionDifference(true);
            setIgnoreRecompression(true);
            setMinScore(0.7);
        }
    };

    const handleSave = async () => {
        try {
            await duplicatesApi.updateDuplicateRuleSet({
                id: selectedProfile() === 'custom' ? 'custom-match' : selectedProfile(),
                name: 'Custom Rules',
                description: 'User defined duplicate rules',
                consider_exact_match: considerExactMatch(),
                consider_visual_match: considerVisualMatch(),
                consider_crop_match: considerCropMatch(),
                ignore_resolution_difference: ignoreResolutionDifference(),
                ignore_recompression: ignoreRecompression(),
                allow_rotation: false,
                allow_mirroring: false,
                min_score: minScore(),
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString()
            });

            await duplicatesApi.startDuplicateScan();
        } catch (error) {
            console.error('Failed to save and scan duplicate rules', error);
        } finally {
            props.onClose();
        }
    };

    return (
        <Modal
            isOpen={props.isOpen}
            onClose={props.onClose}
            title="Duplicate Detection Rules"
            size="md"
            footer={
                <div class="duplicate-rules-modal-footer">
                    <Button variant="ghost" onClick={props.onClose}>
                        Cancel
                    </Button>
                    <Button variant="primary" onClick={handleSave}>
                        Save and Rescan
                    </Button>
                </div>
            }
        >
            <div class="duplicate-rules-modal-content">
                <div class="duplicate-rules-section">
                    <label class="duplicate-rules-label">Detection Profile</label>
                    <Select
                        options={profiles}
                        value={selectedProfile()}
                        onValueChange={handleProfileChange}
                        placeholder="Select a predefined profile"
                    />
                </div>

                <div class="duplicate-rules-section">
                    <h3 class="duplicate-rules-section-title">Detection Types</h3>

                    <div class="duplicate-rules-row">
                        <div class="duplicate-rules-row-text">
                            <span class="duplicate-rules-row-label">Exact Match</span>
                            <span class="duplicate-rules-row-desc">
                                Finds identical files using block-level hash.
                            </span>
                        </div>
                        <Switch
                            checked={considerExactMatch()}
                            onCheckedChange={setConsiderExactMatch}
                            disabled={selectedProfile() !== 'custom'}
                        />
                    </div>

                    <div class="duplicate-rules-row">
                        <div class="duplicate-rules-row-text">
                            <span class="duplicate-rules-row-label">Visual Match</span>
                            <span class="duplicate-rules-row-desc">
                                Finds visually similar files using perceptual hashing.
                            </span>
                        </div>
                        <Switch
                            checked={considerVisualMatch()}
                            onCheckedChange={setConsiderVisualMatch}
                            disabled={selectedProfile() !== 'custom'}
                        />
                    </div>

                    <div class="duplicate-rules-row">
                        <div class="duplicate-rules-row-text">
                            <span class="duplicate-rules-row-label">Derived / Edited Match</span>
                            <span class="duplicate-rules-row-desc">
                                Finds files that are cropped or partially edited.
                            </span>
                        </div>
                        <Switch
                            checked={considerCropMatch()}
                            onCheckedChange={setConsiderCropMatch}
                            disabled={selectedProfile() !== 'custom'}
                        />
                    </div>
                </div>

                <div class="duplicate-rules-section">
                    <h3 class="duplicate-rules-section-title">Tolerance Options</h3>

                    <div class="duplicate-rules-row">
                        <div class="duplicate-rules-row-text">
                            <span class="duplicate-rules-row-label">
                                Ignore resolution differences
                            </span>
                        </div>
                        <Switch
                            checked={ignoreResolutionDifference()}
                            onCheckedChange={setIgnoreResolutionDifference}
                            disabled={selectedProfile() !== 'custom'}
                        />
                    </div>

                    <div class="duplicate-rules-row">
                        <div class="duplicate-rules-row-text">
                            <span class="duplicate-rules-row-label">
                                Ignore recompression (JPEG/PNG)
                            </span>
                        </div>
                        <Switch
                            checked={ignoreRecompression()}
                            onCheckedChange={setIgnoreRecompression}
                            disabled={selectedProfile() !== 'custom'}
                        />
                    </div>

                    <div class="duplicate-rules-row-slider">
                        <div class="duplicate-rules-row-text">
                            <span class="duplicate-rules-row-label">
                                Similarity Threshold ({(minScore() * 100).toFixed(0)}%)
                            </span>
                            <span class="duplicate-rules-row-desc">
                                Lower values find more duplicates but increase false positives.
                            </span>
                        </div>
                        <Slider
                            value={minScore()}
                            minimumValue={0.5}
                            maximumValue={1.0}
                            stepValue={0.01}
                            onValueChange={setMinScore}
                            isDisabled={selectedProfile() !== 'custom'}
                        />
                    </div>
                </div>

                <div class="duplicate-rules-info">
                    <Info size={16} />
                    <span>
                        Applying new rules requires a full re-scan of the duplicate fingerprints.
                    </span>
                </div>
            </div>
        </Modal>
    );
};
