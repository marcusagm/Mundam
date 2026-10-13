import { Component, createSignal, onMount } from 'solid-js';
import { Button, SectionGroup, Switch } from '../../ui';
import { DuplicateRulesForm } from '../duplicates/DuplicateRulesForm';
import { useDuplicateRules } from '../duplicates/hooks/useDuplicateRules';
import { duplicatesApi } from '../../../lib/duplicates';
import { useNotification, useSettings } from '../../../core/hooks';

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
            <p style={{ 'margin-bottom': '24px', color: 'var(--text-secondary)' }}>
                Configure how Mundam scans, groups, and identifies duplicate media assets.
            </p>

            <SectionGroup
                title="Background Scanning"
                description="Settings related to real-time exact duplicate detection on file import."
            >
                <div
                    class="general-setting-row"
                    style={{
                        display: 'flex',
                        'justify-content': 'space-between',
                        'align-items': 'center'
                    }}
                >
                    <div style={{ display: 'flex', 'flex-direction': 'column', gap: '4px' }}>
                        <span class="setting-label">Auto-Detect Exact Duplicates on Import</span>
                        <span style={{ color: 'var(--text-tertiary)', 'font-size': '0.85rem' }}>
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
                <div class="general-setting-row">
                    <DuplicateRulesForm
                        selectedProfile={duplicateRules.selectedProfile()}
                        considerExactMatch={duplicateRules.considerExactMatch()}
                        considerVisualMatch={duplicateRules.considerVisualMatch()}
                        considerCropMatch={duplicateRules.considerCropMatch()}
                        ignoreResolutionDifference={duplicateRules.ignoreResolutionDifference()}
                        ignoreRecompression={duplicateRules.ignoreRecompression()}
                        minimumScore={duplicateRules.minimumScore()}
                        isDisabled={duplicateRules.isSaving()}
                        onProfileChange={duplicateRules.handleProfileChange}
                        onConsiderExactMatchChange={duplicateRules.setConsiderExactMatch}
                        onConsiderVisualMatchChange={duplicateRules.setConsiderVisualMatch}
                        onConsiderCropMatchChange={duplicateRules.setConsiderCropMatch}
                        onIgnoreResolutionDifferenceChange={
                            duplicateRules.setIgnoreResolutionDifference
                        }
                        onIgnoreRecompressionChange={duplicateRules.setIgnoreRecompression}
                        onMinimumScoreChange={duplicateRules.setMinimumScore}
                    />
                </div>

                <div
                    style={{
                        display: 'flex',
                        'justify-content': 'flex-end',
                        'margin-top': '16px'
                    }}
                >
                    <Button
                        variant="primary"
                        onClick={handleSaveRules}
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
                <div class="general-setting-row">
                    <Button
                        variant="destructive"
                        size="sm"
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
