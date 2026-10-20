import { Component, Show } from 'solid-js';
import { Select } from '../../ui/Select';
import { Slider } from '../../ui/Slider';
import { Switch } from '../../ui/Switch';
import { Alert } from '../../ui/Alert';
import { DuplicateRuleProfile } from './types';
import './duplicate-rules-modal.css';

/**
 * Properties for the DuplicateRulesForm component.
 */
export interface DuplicateRulesFormProperties {
    /** Currently selected profile identifier ('exact-match', 'visual-match', 'aggressive', 'custom') */
    selectedProfile: DuplicateRuleProfile;
    /** Whether block-level SHA-256 exact matching is active */
    considerExactMatch: boolean;
    /** Whether perceptual gradient visual matching is active */
    considerVisualMatch: boolean;
    /** Whether 4x4 spatial block hash crop matching is active */
    considerCropMatch: boolean;
    /** Whether resolution differences are ignored */
    ignoreResolutionDifference: boolean;
    /** Whether format recompressions are ignored */
    ignoreRecompression: boolean;
    /** Whether rotated (90°/180°/270°) hash variants are tested */
    allowRotation: boolean;
    /** Whether horizontally mirrored hash variants are tested */
    allowMirroring: boolean;
    /** Current shared similarity threshold (0.50 to 1.00) */
    minimumScore: number;
    /** Current visual match threshold (0.50 to 1.00) */
    minimumVisualScore: number;
    /** Current crop match threshold (0.50 to 1.00) */
    minimumCropScore: number;
    /** Whether controls are disabled */
    isDisabled?: boolean;
    /** Callback when profile changes */
    onProfileChange: (profileIdentifier: string) => void;
    /** Callback when exact match toggle changes */
    onConsiderExactMatchChange: (checked: boolean) => void;
    /** Callback when visual match toggle changes */
    onConsiderVisualMatchChange: (checked: boolean) => void;
    /** Callback when crop match toggle changes */
    onConsiderCropMatchChange: (checked: boolean) => void;
    /** Callback when ignore resolution toggle changes */
    onIgnoreResolutionDifferenceChange: (checked: boolean) => void;
    /** Callback when ignore recompression toggle changes */
    onIgnoreRecompressionChange: (checked: boolean) => void;
    /** Callback when rotation toggle changes */
    onAllowRotationChange: (checked: boolean) => void;
    /** Callback when mirroring toggle changes */
    onAllowMirroringChange: (checked: boolean) => void;
    /** Callback when shared similarity threshold slider changes */
    onMinimumScoreChange: (score: number) => void;
    /** Callback when visual threshold slider changes */
    onMinimumVisualScoreChange: (score: number) => void;
    /** Callback when crop threshold slider changes */
    onMinimumCropScoreChange: (score: number) => void;
}

const DETECTION_PROFILES = [
    {
        label: 'Exact Match Only',
        value: 'exact-match',
        description: 'Only flags identical files with matching SHA-256 hashes.'
    },
    {
        label: 'Visual Similarity (Default)',
        value: 'visual-match',
        description: 'Detects recompressed, resized, and visually identical images.'
    },
    {
        label: 'Aggressive Detection',
        value: 'aggressive',
        description: 'Catches crops, partial edits, severe rotations, and modified images.'
    },
    {
        label: 'Custom Rules',
        value: 'custom',
        description: 'Fine-tune each algorithm, tolerance, and threshold individually.'
    }
];

interface FormSectionProperties {
    componentProperties: DuplicateRulesFormProperties;
    isControlsDisabled: boolean;
}

const DetectionAlgorithmsSection: Component<FormSectionProperties> = sectionProperties => {
    return (
        <div class="duplicate-rules-section">
            <h3 class="duplicate-rules-section-title">Detection Types & Algorithms</h3>

            <div class="duplicate-rules-row">
                <div class="duplicate-rules-row-text">
                    <span class="duplicate-rules-row-label">Exact Match</span>
                    <span class="duplicate-rules-row-desc">
                        Finds identical files using block-level SHA-256 content hash.
                    </span>
                </div>
                <Switch
                    checked={sectionProperties.componentProperties.considerExactMatch}
                    onCheckedChange={
                        sectionProperties.componentProperties.onConsiderExactMatchChange
                    }
                    disabled={sectionProperties.isControlsDisabled}
                />
            </div>

            <div class="duplicate-rules-row">
                <div class="duplicate-rules-row-text">
                    <span class="duplicate-rules-row-label">Visual Match</span>
                    <span class="duplicate-rules-row-desc">
                        Finds visually similar files using perceptual dHash and multi-scale texture
                        analysis.
                    </span>
                </div>
                <Switch
                    checked={sectionProperties.componentProperties.considerVisualMatch}
                    onCheckedChange={
                        sectionProperties.componentProperties.onConsiderVisualMatchChange
                    }
                    disabled={sectionProperties.isControlsDisabled}
                />
            </div>

            <div class="duplicate-rules-row">
                <div class="duplicate-rules-row-text">
                    <span class="duplicate-rules-row-label">Derived / Crop Match</span>
                    <span class="duplicate-rules-row-desc">
                        Finds cropped or partially edited files using 4x4 spatial block hash
                        sub-grids.
                    </span>
                </div>
                <Switch
                    checked={sectionProperties.componentProperties.considerCropMatch}
                    onCheckedChange={
                        sectionProperties.componentProperties.onConsiderCropMatchChange
                    }
                    disabled={sectionProperties.isControlsDisabled}
                />
            </div>
        </div>
    );
};

const ToleranceOptionsSection: Component<FormSectionProperties> = sectionProperties => {
    return (
        <div class="duplicate-rules-section">
            <h3 class="duplicate-rules-section-title">Tolerance Options</h3>

            <div class="duplicate-rules-row">
                <div class="duplicate-rules-row-text">
                    <span class="duplicate-rules-row-label">Ignore resolution differences</span>
                    <span class="duplicate-rules-row-desc">
                        Matches images that are identical in composition but have different
                        dimensions.
                    </span>
                </div>
                <Switch
                    checked={sectionProperties.componentProperties.ignoreResolutionDifference}
                    onCheckedChange={
                        sectionProperties.componentProperties.onIgnoreResolutionDifferenceChange
                    }
                    disabled={sectionProperties.isControlsDisabled}
                />
            </div>

            <div class="duplicate-rules-row">
                <div class="duplicate-rules-row-text">
                    <span class="duplicate-rules-row-label">Ignore recompression (JPEG/PNG)</span>
                    <span class="duplicate-rules-row-desc">
                        Permits minor compression artifacts introduced by re-encoding.
                    </span>
                </div>
                <Switch
                    checked={sectionProperties.componentProperties.ignoreRecompression}
                    onCheckedChange={
                        sectionProperties.componentProperties.onIgnoreRecompressionChange
                    }
                    disabled={sectionProperties.isControlsDisabled}
                />
            </div>

            <div class="duplicate-rules-row">
                <div class="duplicate-rules-row-text">
                    <span class="duplicate-rules-row-label">
                        Detect rotations (90° / 180° / 270°)
                    </span>
                    <span class="duplicate-rules-row-desc">
                        Matches images that have been rotated by orthogonal angles.
                    </span>
                </div>
                <Switch
                    checked={sectionProperties.componentProperties.allowRotation}
                    onCheckedChange={sectionProperties.componentProperties.onAllowRotationChange}
                    disabled={sectionProperties.isControlsDisabled}
                />
            </div>

            <div class="duplicate-rules-row">
                <div class="duplicate-rules-row-text">
                    <span class="duplicate-rules-row-label">Detect mirrored (flipped) images</span>
                    <span class="duplicate-rules-row-desc">
                        Matches images that have been horizontally flipped.
                    </span>
                </div>
                <Switch
                    checked={sectionProperties.componentProperties.allowMirroring}
                    onCheckedChange={sectionProperties.componentProperties.onAllowMirroringChange}
                    disabled={sectionProperties.isControlsDisabled}
                />
            </div>
        </div>
    );
};

const SimilarityThresholdsSection: Component<FormSectionProperties> = sectionProperties => {
    const globalPercentage = () =>
        (sectionProperties.componentProperties.minimumScore * 100).toFixed(0);
    const visualPercentage = () =>
        (sectionProperties.componentProperties.minimumVisualScore * 100).toFixed(0);
    const cropPercentage = () =>
        (sectionProperties.componentProperties.minimumCropScore * 100).toFixed(0);

    return (
        <div class="duplicate-rules-section">
            <h3 class="duplicate-rules-section-title">Similarity Thresholds</h3>

            <div class="duplicate-rules-row-slider">
                <div class="duplicate-rules-row-text">
                    <span class="duplicate-rules-row-label">
                        Global Threshold ({globalPercentage()}%)
                    </span>
                    <span class="duplicate-rules-row-desc">
                        Shared fallback threshold. Used when independent thresholds are not set.
                    </span>
                </div>
                <Slider
                    value={sectionProperties.componentProperties.minimumScore}
                    minimumValue={0.5}
                    maximumValue={1.0}
                    stepValue={0.01}
                    showTicks={false}
                    onValueChange={sectionProperties.componentProperties.onMinimumScoreChange}
                    isDisabled={sectionProperties.isControlsDisabled}
                />
            </div>

            <Show when={sectionProperties.componentProperties.considerVisualMatch}>
                <div class="duplicate-rules-row-slider">
                    <div class="duplicate-rules-row-text">
                        <span class="duplicate-rules-row-label">
                            Visual Match Threshold ({visualPercentage()}%)
                        </span>
                        <span class="duplicate-rules-row-desc">
                            Minimum perceptual similarity required for visual matches. Lower values
                            find more near-duplicates but increase false positives.
                        </span>
                    </div>
                    <Slider
                        value={sectionProperties.componentProperties.minimumVisualScore}
                        minimumValue={0.5}
                        maximumValue={1.0}
                        stepValue={0.01}
                        showTicks={false}
                        onValueChange={
                            sectionProperties.componentProperties.onMinimumVisualScoreChange
                        }
                        isDisabled={sectionProperties.isControlsDisabled}
                    />
                </div>
            </Show>

            <Show when={sectionProperties.componentProperties.considerCropMatch}>
                <div class="duplicate-rules-row-slider">
                    <div class="duplicate-rules-row-text">
                        <span class="duplicate-rules-row-label">
                            Crop Match Threshold ({cropPercentage()}%)
                        </span>
                        <span class="duplicate-rules-row-desc">
                            Minimum spatial block hash overlap for crop/derivative matches. Lower
                            values detect more aggressive crops.
                        </span>
                    </div>
                    <Slider
                        value={sectionProperties.componentProperties.minimumCropScore}
                        minimumValue={0.5}
                        maximumValue={1.0}
                        stepValue={0.01}
                        showTicks={false}
                        onValueChange={
                            sectionProperties.componentProperties.onMinimumCropScoreChange
                        }
                        isDisabled={sectionProperties.isControlsDisabled}
                    />
                </div>
            </Show>
        </div>
    );
};

/**
 * Controlled form allowing configuration of duplicate detection rules.
 *
 * Supports selecting predefined profiles ('exact-match', 'visual-match', 'aggressive')
 * or fine-tuning each toggle and slider individually under the 'custom' profile.
 *
 * @param {DuplicateRulesFormProperties} componentProperties - The form input properties.
 * @returns {JSX.Element} The rendered form elements.
 */
export const DuplicateRulesForm: Component<DuplicateRulesFormProperties> = componentProperties => {
    const isCustomMode = () => componentProperties.selectedProfile === 'custom';
    const isControlsDisabled = () => Boolean(componentProperties.isDisabled) || !isCustomMode();

    return (
        <div class="duplicate-rules-modal-content">
            <div class="duplicate-rules-section">
                <label class="duplicate-rules-label">Detection Profile</label>
                <Select
                    options={DETECTION_PROFILES}
                    value={componentProperties.selectedProfile}
                    onValueChange={componentProperties.onProfileChange}
                    placeholder="Select a predefined profile"
                    disabled={componentProperties.isDisabled}
                />
            </div>

            <DetectionAlgorithmsSection
                componentProperties={componentProperties}
                isControlsDisabled={isControlsDisabled()}
            />

            <ToleranceOptionsSection
                componentProperties={componentProperties}
                isControlsDisabled={isControlsDisabled()}
            />

            <SimilarityThresholdsSection
                componentProperties={componentProperties}
                isControlsDisabled={isControlsDisabled()}
            />

            <Alert variant="info">
                Applying new rules sets the active policy for duplicate grouping. Rescanning
                leverages pre-computed fingerprints to group assets without re-extracting image
                files.
            </Alert>
        </div>
    );
};
