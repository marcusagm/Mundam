import { createSignal, type Component } from 'solid-js';
import { SectionGroup } from '../../ui/SectionGroup';
import { Select } from '../../ui/Select';
import type { SelectOption } from '../../ui/Select/types';
import { useSettings } from '../../../core/hooks';
import type { SettingsPayload } from '../../../core/store/settings/schemas';
import './performance-settings.css';

const budgetOptions = [
    { value: '0', label: 'Automatic (Recommended)' },
    { value: '1', label: '1 operation' },
    { value: '2', label: '2 operations' },
    { value: '4', label: '4 operations' },
    { value: '8', label: '8 operations' }
];
const thumbnailOptions = [
    { value: '0', label: 'Use shared limit' },
    { value: '1', label: '1 operation' },
    { value: '2', label: '2 operations' },
    { value: '4', label: '4 operations' },
    { value: '8', label: '8 operations' }
];
const indexerOptions = [
    { value: '50', label: '50 (HDD / SD)' },
    { value: '100', label: '100 (Balanced)' },
    { value: '200', label: '200 (SSD)' },
    { value: '300', label: '300 (Aggressive)' },
    { value: '400', label: '400 (NVMe)' }
];

/** Keeps persisted custom limits visible until the user selects another value. */
function configuredOptions(options: SelectOption[], value: number): SelectOption[] {
    if (options.some(option => option.value === String(value))) return options;
    return [...options, { value: String(value), label: `${value} (Saved setting)` }];
}

/** Controls live media admission independently of the indexer's startup limit. */
export const PerformanceSettings: Component = () => {
    const settings = useSettings();
    const [isSaving, setIsSaving] = createSignal(false);
    const [feedbackMessage, setFeedbackMessage] = createSignal('');

    /** Persists before acknowledging the effective runtime policy change. */
    const save = async (payload: SettingsPayload, requiresRestart = false): Promise<void> => {
        if (isSaving()) return;
        setIsSaving(true);
        setFeedbackMessage('Saving performance settings…');
        try {
            const result = await settings.updateSettings(payload);
            if (!result.success) {
                setFeedbackMessage('Could not save performance settings. Please try again.');
                return;
            }
            setFeedbackMessage(
                requiresRestart
                    ? 'Indexer setting saved. Restart the app to apply it.'
                    : 'Processing limits saved. Current operations finish normally; new operations use the updated limits.'
            );
        } catch {
            setFeedbackMessage('Could not save performance settings. Please try again.');
        } finally {
            setIsSaving(false);
        }
    };

    return (
        <SectionGroup
            title="Performance"
            description="Thumbnails, colors and duplicate analysis share processing capacity. Higher limits can increase CPU and disk usage."
        >
            <div class="performance-settings-row">
                <span class="performance-settings-label" id="shared-processing-label">
                    Shared processing limit:
                </span>
                <div class="performance-settings-select">
                    <Select
                        aria-labelledby="shared-processing-label"
                        options={configuredOptions(budgetOptions, settings.processingConcurrency())}
                        value={String(settings.processingConcurrency())}
                        disabled={isSaving()}
                        onValueChange={value => void save({ processingConcurrency: Number(value) })}
                    />
                </div>
            </div>
            <div class="performance-settings-row">
                <span class="performance-settings-label" id="thumbnail-processing-label">
                    Thumbnail limit:
                </span>
                <div class="performance-settings-select">
                    <Select
                        aria-labelledby="thumbnail-processing-label"
                        options={configuredOptions(thumbnailOptions, settings.thumbnailThreads())}
                        value={String(settings.thumbnailThreads())}
                        disabled={isSaving()}
                        onValueChange={value => void save({ thumbnailThreads: Number(value) })}
                    />
                </div>
            </div>
            <p class="performance-settings-note">
                Media limits apply without restarting. The thumbnail limit cannot exceed shared
                capacity. See effective limits and pause workers in System Activity. These settings
                limit operations; extractors may use multiple CPU threads.
            </p>
            <div class="performance-settings-row">
                <span class="performance-settings-label" id="indexer-processing-label">
                    Indexer concurrency:
                </span>
                <div class="performance-settings-select">
                    <Select
                        aria-labelledby="indexer-processing-label"
                        options={configuredOptions(
                            indexerOptions,
                            settings.indexerConcurrencyLimit()
                        )}
                        value={String(settings.indexerConcurrencyLimit())}
                        disabled={isSaving()}
                        onValueChange={value =>
                            void save({ indexerConcurrencyLimit: Number(value) }, true)
                        }
                    />
                </div>
            </div>
            <p class="performance-settings-note">Indexer concurrency requires an app restart.</p>
            <p role="status" aria-live="polite" class="performance-settings-status">
                {feedbackMessage()}
            </p>
        </SectionGroup>
    );
};
