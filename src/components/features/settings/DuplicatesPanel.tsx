import { Component, createSignal, onMount } from 'solid-js';
import { Button, SectionGroup, Switch } from '../../ui';
import { DuplicateRulesForm } from '../duplicates/DuplicateRulesForm';
import { useDuplicateRules } from '../duplicates/hooks/useDuplicateRules';
import { duplicatesApi } from '../../../lib/duplicates';
import { useNotification, useSettings } from '../../../core/hooks';
import './duplicates-panel.css';

/**
 * Settings panel for configuring Duplicate File Detection rules, background workers, and index maintenance.
 *
 * @returns {JSX.Element} The rendered settings panel.
 */
export const DuplicatesPanel: Component = () => {
    const settings = useSettings();
    const notification = useNotification();
    const duplicateRules = useDuplicateRules();

    const [isRebuildingIndex, setIsRebuildingIndex] = createSignal<boolean>(false);

    onMount(async () => {
        await settings.initialize();
    });

    const handleAutoDetectChange = async (checked: boolean): Promise<void> => {
        const updateResult = await settings.updateSettings({ autoDetectDuplicates: checked });
        if (updateResult.success) {
            notification.success('Settings Saved', 'Background auto-detection preference saved.');
        } else {
            notification.error('Error', updateResult.error.message);
        }
    };

    const handleSaveRules = async (): Promise<void> => {
        try {
            await duplicateRules.saveRules();
            notification.success(
                'Rules Saved',
                'Duplicate detection rules successfully updated and set as active configuration.'
            );
        } catch (error) {
            console.error('Failed to save duplicate detection rules:', error);
            notification.error('Error', 'Failed to save duplicate detection rules.');
        }
    };

    const handleRebuildIndex = async (): Promise<void> => {
        setIsRebuildingIndex(true);
        notification.info(
            'Rebuilding Index',
            'Resetting existing duplicate index records and launching a library-wide scan...'
        );
        try {
            await duplicatesApi.rebuildDuplicateIndex();
            notification.success(
                'Index Reset Complete',
                'Duplicate index has been cleared and a fresh background scan has started.'
            );
        } catch (error) {
            console.error('Failed to rebuild duplicate index:', error);
            notification.error('Error', 'Failed to rebuild duplicate index.');
        } finally {
            setIsRebuildingIndex(false);
        }
    };

    return (
        <div class="settings-panel-content duplicates-panel">
            <h2 class="settings-panel-title">Duplicate Detection</h2>
            <p class="settings-panel-description">
                Configure how Mundam scans, groups, and identifies duplicate media assets.
            </p>

            <SectionGroup
                title="Background Scanning"
                description="Settings related to real-time exact duplicate detection on file import."
            >
                <div class="general-setting-row duplicates-setting-row">
                    <div class="duplicates-setting-label-block">
                        <span class="setting-label">Auto-Detect Exact Duplicates on Import</span>
                        <span class="duplicates-setting-description">
                            Instantly check and group exact copies as soon as they are added to the
                            library.
                        </span>
                    </div>
                    <Switch
                        checked={settings.autoDetectDuplicates()}
                        onCheckedChange={handleAutoDetectChange}
                    />
                </div>
            </SectionGroup>

            <SectionGroup
                title="Detection Rules & Algorithms"
                description="Active detection profiles, algorithm toggles, and similarity tolerance thresholds."
            >
                <div class="duplicates-rules-container">
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
                        onIgnoreResolutionDifferenceChange={
                            duplicateRules.setIgnoreResolutionDifference
                        }
                        onIgnoreRecompressionChange={duplicateRules.setIgnoreRecompression}
                        onAllowRotationChange={duplicateRules.setAllowRotation}
                        onAllowMirroringChange={duplicateRules.setAllowMirroring}
                        onMinimumScoreChange={duplicateRules.setMinimumScore}
                        onMinimumVisualScoreChange={duplicateRules.setMinimumVisualScore}
                        onMinimumCropScoreChange={duplicateRules.setMinimumCropScore}
                    />
                </div>

                <div class="setting-action-row duplicates-rules-action-row">
                    <div class="duplicates-rules-status">
                        {duplicateRules.hasUnsavedChanges() ? (
                            <span class="duplicates-status-badge duplicates-status-badge--dirty">
                                <span class="duplicates-status-dot" />
                                Unsaved changes (click to apply)
                            </span>
                        ) : (
                            <span class="duplicates-status-badge duplicates-status-badge--clean">
                                <span class="duplicates-status-dot" />
                                Rules active
                            </span>
                        )}
                    </div>
                    <Button
                        variant="primary"
                        onClick={handleSaveRules}
                        disabled={!duplicateRules.hasUnsavedChanges()}
                        loading={duplicateRules.isSaving()}
                    >
                        Save Detection Rules
                    </Button>
                </div>
            </SectionGroup>

            <SectionGroup
                title="Maintenance"
                description="Erase all existing duplicate fingerprints and groups to re-analyze the entire library from scratch."
            >
                <div class="setting-action-row">
                    <Button
                        variant="destructive"
                        onClick={handleRebuildIndex}
                        loading={isRebuildingIndex()}
                    >
                        Rebuild Index
                    </Button>
                </div>
            </SectionGroup>
        </div>
    );
};
