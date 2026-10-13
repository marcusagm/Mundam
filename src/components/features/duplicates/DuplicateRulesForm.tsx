import { Component } from 'solid-js';
import { Select } from '../../ui/Select';
import { Slider } from '../../ui/Slider';
import { Switch } from '../../ui/Switch';
import { Alert } from '../../ui/Alert';
import { DuplicateRuleProfile } from './types';
import './duplicate-rules-modal.css';

export interface DuplicateRulesFormProperties {
    /** Currently selected profile identifier */
    selectedProfile: DuplicateRuleProfile;
    /** Whether exact SHA-256 content match is enabled */
    considerExactMatch: boolean;
    /** Whether visual perceptual match is enabled */
    considerVisualMatch: boolean;
    /** Whether spatial block hash crop match is enabled */
    considerCropMatch: boolean;
    /** Whether resolution variations are ignored */
    ignoreResolutionDifference: boolean;
    /** Whether format recompressions are ignored */
    ignoreRecompression: boolean;
    /** Current similarity threshold (0.50 to 1.00) */
    minimumScore: number;
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
    /** Callback when similarity threshold slider changes */
    onMinimumScoreChange: (score: number) => void;
}

const DETECTION_PROFILES = [
    { value: 'exact-match', label: 'Strict Exact Match' },
    { value: 'visual-match', label: 'Visual Similarity (Default)' },
    { value: 'aggressive', label: 'Aggressive Deduplication' },
    { value: 'custom', label: 'Custom Rules' }
];

/**
 * Shared form component for tuning duplicate detection algorithm parameters and profiles.
 * Reused inside DuplicateRulesModal and DuplicatesPanel for maximum consistency.
 *
 * @param {DuplicateRulesFormProperties} componentProperties - Reactive component properties.
 * @returns {JSX.Element} The rendered form elements.
 */
export const DuplicateRulesForm: Component<DuplicateRulesFormProperties> = componentProperties => {
    const isCustomMode = () => componentProperties.selectedProfile === 'custom';

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
                        checked={componentProperties.considerExactMatch}
                        onCheckedChange={componentProperties.onConsiderExactMatchChange}
                        disabled={componentProperties.isDisabled || !isCustomMode()}
                    />
                </div>

                <div class="duplicate-rules-row">
                    <div class="duplicate-rules-row-text">
                        <span class="duplicate-rules-row-label">Visual Match</span>
                        <span class="duplicate-rules-row-desc">
                            Finds visually similar files using perceptual dHash and multi-scale
                            texture analysis.
                        </span>
                    </div>
                    <Switch
                        checked={componentProperties.considerVisualMatch}
                        onCheckedChange={componentProperties.onConsiderVisualMatchChange}
                        disabled={componentProperties.isDisabled || !isCustomMode()}
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
                        checked={componentProperties.considerCropMatch}
                        onCheckedChange={componentProperties.onConsiderCropMatchChange}
                        disabled={componentProperties.isDisabled || !isCustomMode()}
                    />
                </div>
            </div>

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
                        checked={componentProperties.ignoreResolutionDifference}
                        onCheckedChange={componentProperties.onIgnoreResolutionDifferenceChange}
                        disabled={componentProperties.isDisabled || !isCustomMode()}
                    />
                </div>

                <div class="duplicate-rules-row">
                    <div class="duplicate-rules-row-text">
                        <span class="duplicate-rules-row-label">
                            Ignore recompression (JPEG/PNG)
                        </span>
                        <span class="duplicate-rules-row-desc">
                            Permits minor compression artifacts introduced by re-encoding.
                        </span>
                    </div>
                    <Switch
                        checked={componentProperties.ignoreRecompression}
                        onCheckedChange={componentProperties.onIgnoreRecompressionChange}
                        disabled={componentProperties.isDisabled || !isCustomMode()}
                    />
                </div>

                <div class="duplicate-rules-row-slider">
                    <div class="duplicate-rules-row-text">
                        <span class="duplicate-rules-row-label">
                            Similarity Threshold (
                            {(componentProperties.minimumScore * 100).toFixed(0)}%)
                        </span>
                        <span class="duplicate-rules-row-desc">
                            Lower values find more near-duplicates but increase false positives.
                        </span>
                    </div>
                    <Slider
                        value={componentProperties.minimumScore}
                        minimumValue={0.5}
                        maximumValue={1.0}
                        stepValue={0.01}
                        showTicks={false}
                        onValueChange={componentProperties.onMinimumScoreChange}
                        isDisabled={componentProperties.isDisabled || !isCustomMode()}
                    />
                </div>
            </div>

            <Alert variant="info">
                Applying new rules sets the active policy for duplicate grouping. Rescanning
                leverages pre-computed fingerprints to group assets without re-extracting image
                files.
            </Alert>
        </div>
    );
};
