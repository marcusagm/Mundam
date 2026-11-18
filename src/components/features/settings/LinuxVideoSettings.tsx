import { createSignal, onMount, type Component } from 'solid-js';
import { SectionGroup } from '../../ui/SectionGroup';
import { Select } from '../../ui/Select';
import { useSettings } from '../../../core/hooks';
import './performance-settings.css';

/** Offers an opt-in video compatibility preference applied on the next Linux startup. */
export const LinuxVideoSettings: Component = () => {
    const settings = useSettings();
    const [ready, setReady] = createSignal(false);
    const [saving, setSaving] = createSignal(false);
    const [feedback, setFeedback] = createSignal('');

    onMount(async () => {
        try {
            await settings.initialize();
            setReady(true);
        } catch {
            setFeedback('Could not load video settings. Reopen Settings to try again.');
        }
    });

    /**
     * Persists the selected mode without claiming the current renderer changed.
     * @param mode - User-selected normal or compatibility playback mode.
     * @returns Resolves after persistence and inline feedback.
     */
    const saveMode = async (mode: string): Promise<void> => {
        if (saving() || !ready()) return;
        setSaving(true);
        setFeedback('Saving video settings…');
        try {
            const result = await settings.updateSettings({
                linuxVideoCompatibility: mode === 'compatibility'
            });
            setFeedback(
                result.success
                    ? 'Video mode saved. Restart Mundam to apply the change.'
                    : 'Could not save video mode. Please try again.'
            );
        } catch {
            setFeedback('Could not save video mode. Please try again.');
        } finally {
            setSaving(false);
        }
    };

    return (
        <SectionGroup
            title="Video playback"
            description="Choose Compatibility if videos have sound but show a black image or visual glitches."
        >
            <div class="performance-settings-row">
                <span class="performance-settings-label" id="linux-video-mode-label">
                    Video mode:
                </span>
                <div class="performance-settings-select">
                    <Select
                        aria-labelledby="linux-video-mode-label"
                        options={[
                            { value: 'normal', label: 'Normal (Default)' },
                            { value: 'compatibility', label: 'Compatibility' }
                        ]}
                        value={settings.linuxVideoCompatibility() ? 'compatibility' : 'normal'}
                        disabled={saving() || !ready()}
                        onValueChange={mode => void saveMode(mode)}
                    />
                </div>
            </div>
            <p class="performance-settings-note">
                Compatibility changes video rendering and may reduce performance. Restart Mundam
                after changing this setting.
            </p>
            <p role="status" aria-live="polite" class="performance-settings-status">
                {feedback()}
            </p>
        </SectionGroup>
    );
};
