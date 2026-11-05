import { Button, SectionGroup } from '../../ui';
import { Component, createSignal, onMount } from 'solid-js';
import { Select } from '../../ui/Select';
import { useNotification, useMetadata } from '../../../core/hooks';
import { invokeCommand } from '../../../lib/api';
import { TrashEmptyModal } from './TrashEmptyModal';
import { openPath } from '@tauri-apps/plugin-opener';

/**
 * Self-contained settings section for Trash management.
 *
 * Handles auto-empty configuration (enabled/disabled toggle + retention days)
 * and manual "Empty Trash" with confirmation modal.
 *
 * Extracted from GeneralPanel to keep each component under 300 lines.
 */
export const TrashSettingsSection: Component = () => {
    const notification = useNotification();
    const metadata = useMetadata();

    const [isOpeningRecovery, setIsOpeningRecovery] = createSignal(false);

    const [isTrashModalOpen, setIsTrashModalOpen] = createSignal(false);
    const [trashAutoEmptyEnabled, setTrashAutoEmptyEnabled] = createSignal(false);
    const [trashAutoEmptyDays, setTrashAutoEmptyDays] = createSignal(30);

    onMount(async () => {
        const autoEmptyEnabledValue = await invokeCommand<string | null>('get_setting', {
            key: 'trash_auto_empty_enabled'
        });
        const autoEmptyDaysValue = await invokeCommand<string | null>('get_setting', {
            key: 'trash_auto_empty_days'
        });
        if (autoEmptyEnabledValue === 'true') setTrashAutoEmptyEnabled(true);
        if (autoEmptyDaysValue) setTrashAutoEmptyDays(Number(autoEmptyDaysValue));
    });

    /** Exposes storage only from the advanced recovery settings action. */
    const handleOpenRecovery = async (): Promise<void> => {
        if (isOpeningRecovery()) return;
        setIsOpeningRecovery(true);
        try {
            await openPath(await invokeCommand<string>('folder_recovery_directory'));
        } catch {
            notification.error('Could not open folder recovery');
        } finally {
            setIsOpeningRecovery(false);
        }
    };

    const handleOpenTrashModal = () => {
        setIsTrashModalOpen(true);
    };

    const handleTrashAutoEmptyToggle = async (value: string) => {
        const isEnabled = value === 'enabled';
        setTrashAutoEmptyEnabled(isEnabled);
        try {
            await invokeCommand('set_setting', {
                key: 'trash_auto_empty_enabled',
                value: String(isEnabled)
            });
        } catch (error) {
            notification.error('Failed to save auto-empty setting.');
            console.error(error);
        }
    };

    const handleTrashAutoEmptyDaysChange = async (value: string) => {
        const days = Number(value);
        setTrashAutoEmptyDays(days);
        try {
            await invokeCommand('set_setting', {
                key: 'trash_auto_empty_days',
                value: String(days)
            });
        } catch (error) {
            notification.error('Failed to save auto-empty days setting.');
            console.error(error);
        }
    };

    return (
        <SectionGroup
            title="Trash"
            description="Manage deleted items. Items in the trash can be restored or permanently deleted."
        >
            <div class="general-setting-row">
                <span class="setting-label">Auto-empty:</span>
                <div style={{ width: '140px' }}>
                    <Select
                        options={[
                            { value: 'disabled', label: 'Disabled' },
                            { value: 'enabled', label: 'Enabled' }
                        ]}
                        value={trashAutoEmptyEnabled() ? 'enabled' : 'disabled'}
                        onValueChange={handleTrashAutoEmptyToggle}
                        placeholder="Select"
                    />
                </div>
            </div>
            {trashAutoEmptyEnabled() && (
                <div class="general-setting-row">
                    <span class="setting-label">Delete items older than:</span>
                    <div style={{ width: '140px' }}>
                        <Select
                            options={[
                                { value: '7', label: '7 days' },
                                { value: '14', label: '14 days' },
                                { value: '30', label: '30 days' },
                                { value: '60', label: '60 days' },
                                { value: '90', label: '90 days' }
                            ]}
                            value={String(trashAutoEmptyDays())}
                            onValueChange={handleTrashAutoEmptyDaysChange}
                            placeholder="Select days"
                        />
                    </div>
                </div>
            )}
            <div class="setting-action-row">
                <Button
                    onClick={handleOpenTrashModal}
                    variant="destructive"
                    disabled={metadata.stats.trash_assets === 0}
                >
                    Empty Trash ({metadata.stats.trash_assets || 0})
                </Button>
            </div>
            <div class="setting-action-row">
                <Button
                    variant="secondary"
                    disabled={isOpeningRecovery()}
                    onClick={handleOpenRecovery}
                >
                    Open Folder Recovery
                </Button>
                <p class="setting-note">
                    Advanced access to deleted folders. Manually changing these files can prevent
                    Undo. Folder recovery is separate from asset trash auto-empty.
                </p>
            </div>
            <TrashEmptyModal
                isOpen={isTrashModalOpen()}
                onClose={() => setIsTrashModalOpen(false)}
                trashCount={metadata.stats.trash_assets || 0}
            />
        </SectionGroup>
    );
};
