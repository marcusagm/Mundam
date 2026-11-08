import { Component } from 'solid-js';
import { Select } from '../../ui/Select';
import { Switch } from '../../ui/Switch';
import { Alert } from '../../ui/Alert';
import { DuplicateRulesFormProperties } from './types';
import { DuplicateSimilaritySliders } from './DuplicateSimilaritySliders';
import './duplicate-rules-modal.css';

export type { DuplicateRulesFormProperties };

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

            <DuplicateSimilaritySliders
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
