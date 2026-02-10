/**
 * Settings Modal
 * A modal with sidebar navigation for application settings
 */

import { Component, createSignal, For, Show } from 'solid-js';
import { Keyboard, Palette, Settings, Info } from 'lucide-solid';
import { cn } from '../../../lib/utils';
import { Modal } from '../../ui/Modal';
import { KeyboardShortcutsPanel } from './KeyboardShortcutsPanel';
import { GeneralPanel } from './GeneralPanel';
import { AppearancePanel } from './AppearancePanel';
import { FoldersPanel } from './FoldersPanel';
import { AboutPanel } from './AboutPanel';
import './settings-modal.css';

export type SettingsTab = 'general' | 'appearance' | 'keyboard-shortcuts' | 'folders' | 'about';

interface SettingsTabDef {
    id: SettingsTab;
    label: string;
    icon: Component<{ size?: number }>;
}

const SETTINGS_TABS: SettingsTabDef[] = [
    { id: 'general', label: 'General', icon: Settings },
    { id: 'appearance', label: 'Appearance', icon: Palette },
    { id: 'keyboard-shortcuts', label: 'Keyboard Shortcuts', icon: Keyboard },
    // { id: 'folders', label: 'Folders', icon: FolderOpen },
    { id: 'about', label: 'About', icon: Info }
];

export interface SettingsModalProps {
    isOpen: boolean;
    onClose: () => void;
    initialTab?: SettingsTab;
}

export const SettingsModal: Component<SettingsModalProps> = props => {
    const [activeTab, setActiveTab] = createSignal<SettingsTab>(props.initialTab || 'general');

    return (
        <Modal
            isOpen={props.isOpen}
            onClose={props.onClose}
            title="Settings"
            size="xl"
            class="settings-modal-wrapper"
        >
            <div class="settings-modal-content">
                {/* Sidebar */}
                <nav class="settings-sidebar" aria-label="Settings navigation">
                    <ul class="settings-sidebar-list">
                        <For each={SETTINGS_TABS}>
                            {tab => (
                                <li>
                                    <button
                                        type="button"
                                        class={cn(
                                            'settings-sidebar-item',
                                            activeTab() === tab.id && 'is-active'
                                        )}
                                        onClick={() => setActiveTab(tab.id)}
                                        aria-current={activeTab() === tab.id ? 'page' : undefined}
                                    >
                                        <tab.icon size={16} />
                                        <span>{tab.label}</span>
                                    </button>
                                </li>
                            )}
                        </For>
                    </ul>
                </nav>

                {/* Panel Content */}
                <div class="settings-panel">
                    <Show when={activeTab() === 'general'}>
                        <GeneralPanel />
                    </Show>
                    <Show when={activeTab() === 'appearance'}>
                        <AppearancePanel />
                    </Show>
                    <Show when={activeTab() === 'keyboard-shortcuts'}>
                        <KeyboardShortcutsPanel />
                    </Show>
                    <Show when={activeTab() === 'folders'}>
                        <FoldersPanel />
                    </Show>
                    <Show when={activeTab() === 'about'}>
                        <AboutPanel />
                    </Show>
                </div>
            </div>
        </Modal>
    );
};
