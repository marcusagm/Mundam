import { Component, Show } from 'solid-js';
import { Slider } from '../../ui/Slider';
import { DuplicateRulesFormProperties } from './types';

export interface SimilaritySlidersProperties {
    componentProperties: DuplicateRulesFormProperties;
    isControlsDisabled: boolean;
}

/**
 * Renders the slider controls for shared baseline similarity, visual similarity,
 * and crop/derivative overlap tolerances.
 *
 * @param {SimilaritySlidersProperties} properties - Properties for the sliders component.
 * @returns {JSX.Element} The rendered similarity sliders.
 */
export const DuplicateSimilaritySliders: Component<SimilaritySlidersProperties> = properties => {
    const globalPercentage = () => (properties.componentProperties.minimumScore * 100).toFixed(0);
    const visualPercentage = () =>
        (properties.componentProperties.minimumVisualScore * 100).toFixed(0);
    const cropPercentage = () => (properties.componentProperties.minimumCropScore * 100).toFixed(0);

    return (
        <div class="duplicate-rules-section">
            <h3 class="duplicate-rules-section-title">Similarity Thresholds</h3>

            <div class="duplicate-rules-slider-card">
                <div class="duplicate-rules-slider-header">
                    <span class="duplicate-rules-row-label">Global Similarity Threshold</span>
                    <span class="duplicate-rules-percentage-badge">{globalPercentage()}%</span>
                </div>
                <span class="duplicate-rules-row-desc">
                    Shared baseline similarity required to flag files as duplicates. Used when
                    specific independent thresholds are not set.
                </span>
                <div class="duplicate-rules-slider-body">
                    <Slider
                        value={properties.componentProperties.minimumScore}
                        minimumValue={0.5}
                        maximumValue={1.0}
                        stepValue={0.01}
                        showTicks={false}
                        showTooltip={true}
                        formatValue={sliderValue => `${(sliderValue * 100).toFixed(0)}%`}
                        onValueChange={properties.componentProperties.onMinimumScoreChange}
                        isDisabled={properties.isControlsDisabled}
                    />
                    <div class="duplicate-rules-slider-scale">
                        <span>50% (Loose)</span>
                        <span>75% (Balanced)</span>
                        <span>100% (Strict)</span>
                    </div>
                </div>
            </div>

            <Show when={properties.componentProperties.considerVisualMatch}>
                <div class="duplicate-rules-slider-card">
                    <div class="duplicate-rules-slider-header">
                        <span class="duplicate-rules-row-label">Visual Match Threshold</span>
                        <span class="duplicate-rules-percentage-badge">{visualPercentage()}%</span>
                    </div>
                    <span class="duplicate-rules-row-desc">
                        Minimum perceptual similarity required for visual matches. Lower values find
                        more near-duplicates but increase false positives.
                    </span>
                    <div class="duplicate-rules-slider-body">
                        <Slider
                            value={properties.componentProperties.minimumVisualScore}
                            minimumValue={0.5}
                            maximumValue={1.0}
                            stepValue={0.01}
                            showTicks={false}
                            showTooltip={true}
                            formatValue={sliderValue => `${(sliderValue * 100).toFixed(0)}%`}
                            onValueChange={
                                properties.componentProperties.onMinimumVisualScoreChange
                            }
                            isDisabled={properties.isControlsDisabled}
                        />
                        <div class="duplicate-rules-slider-scale">
                            <span>50% (Loose)</span>
                            <span>75% (Balanced)</span>
                            <span>100% (Strict)</span>
                        </div>
                    </div>
                </div>
            </Show>

            <Show when={properties.componentProperties.considerCropMatch}>
                <div class="duplicate-rules-slider-card">
                    <div class="duplicate-rules-slider-header">
                        <span class="duplicate-rules-row-label">Crop & Derivative Threshold</span>
                        <span class="duplicate-rules-percentage-badge">{cropPercentage()}%</span>
                    </div>
                    <span class="duplicate-rules-row-desc">
                        Minimum spatial block hash overlap for crop/derivative matches. Lower values
                        detect more aggressive crops.
                    </span>
                    <div class="duplicate-rules-slider-body">
                        <Slider
                            value={properties.componentProperties.minimumCropScore}
                            minimumValue={0.5}
                            maximumValue={1.0}
                            stepValue={0.01}
                            showTicks={false}
                            showTooltip={true}
                            formatValue={sliderValue => `${(sliderValue * 100).toFixed(0)}%`}
                            onValueChange={properties.componentProperties.onMinimumCropScoreChange}
                            isDisabled={properties.isControlsDisabled}
                        />
                        <div class="duplicate-rules-slider-scale">
                            <span>50% (Loose)</span>
                            <span>75% (Balanced)</span>
                            <span>100% (Strict)</span>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    );
};
