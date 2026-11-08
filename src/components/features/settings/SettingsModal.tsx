/**
 * Settings Modal
 * A modal with structured categorized sidebar navigation for application settings
 */

import { Component, createSignal, For, Show } from 'solid-js';
import { Keyboard, Palette, Settings, Info, Copy, Cpu, HardDrive } from 'lucide-solid';
import { cn } from '../../../lib/utils';
import { Modal } from '../../ui';
import { KeyboardShortcutsPanel } from './KeyboardShortcutsPanel';
import { GeneralPanel } from './GeneralPanel';
import { AppearancePanel } from './AppearancePanel';
import { PerformancePanel } from './PerformancePanel';
import { StoragePanel } from './StoragePanel';
import { FoldersPanel } from './FoldersPanel';
import { DuplicatesPanel } from './DuplicatesPanel';
import { AboutPanel } from './AboutPanel';
import './settings-modal.css';

/**
 * Valid settings tabs.
 */
export type SettingsTab =
    | 'general'
    | 'appearance'
    | 'performance'
    | 'storage'
    | 'keyboard-shortcuts'
    | 'folders'
    | 'duplicates'
    | 'about';

/**
 * Definition structure for a settings tab.
 */
interface SettingsTabDefinition {
    /** Unique identifier for the tab. */
    identifier: SettingsTab;
    /** Human-readable label for the tab. */
    label: string;
    /** Icon component for the tab. */
    icon: Component<{ size?: number }>;
}

/**
 * A group of related settings tabs displayed under a section in the sidebar.
 */
interface SettingsTabCategory {
    /** Group label displayed in the sidebar. */
    categoryLabel: string;
    /** Tabs belonging to this category. */
    tabs: SettingsTabDefinition[];
}

/**
 * Collection of all available settings tabs organized by thematic categories.
 */
const SETTINGS_CATEGORIES: SettingsTabCategory[] = [
    {
        categoryLabel: 'Preferences',
        tabs: [
            { identifier: 'general', label: 'General', icon: Settings },
            { identifier: 'appearance', label: 'Appearance', icon: Palette },
            { identifier: 'keyboard-shortcuts', label: 'Keyboard Shortcuts', icon: Keyboard }
        ]
    },
    {
        categoryLabel: 'System & Data',
        tabs: [
            { identifier: 'performance', label: 'Performance', icon: Cpu },
            { identifier: 'storage', label: 'Storage & Maintenance', icon: HardDrive },
            { identifier: 'duplicates', label: 'Duplicates', icon: Copy }
        ]
    },
    {
        categoryLabel: 'Application',
        tabs: [{ identifier: 'about', label: 'About', icon: Info }]
    }
];

/**
 * Properties for the SettingsModal component.
 */
export interface SettingsModalProperties {
    /** Whether the settings modal is currently open. */
    isOpen: boolean;
    /** Callback invoked when the modal requests closure. */
    onClose: () => void;
    /** The tab to show when the modal first opens. */
    initialTab?: SettingsTab;
}

/**
 * A modal providing access to various application-wide configurations,
 * organized into a sidebar-navigated layout.
 *
 * @param componentProperties - Properties for the component.
 * @returns The rendered SettingsModal.
 */
export const SettingsModal: Component<SettingsModalProperties> = componentProperties => {
    const [activeTab, setActiveTab] = createSignal<SettingsTab>(
        componentProperties.initialTab || 'general'
    );

    return (
        <Modal
            isOpen={componentProperties.isOpen}
            onClose={componentProperties.onClose}
            title="Settings"
            size="xl"
            class="settings-modal-wrapper"
        >
            <div class="settings-modal-content">
                {/* Sidebar */}
                <nav class="settings-sidebar" aria-label="Settings navigation">
                    <For each={SETTINGS_CATEGORIES}>
                        {category => (
                            <div class="settings-sidebar-category">
                                <span class="settings-sidebar-category-header">
                                    {category.categoryLabel}
                                </span>
                                <ul class="settings-sidebar-list">
                                    <For each={category.tabs}>
                                        {tab => (
                                            <li>
                                                <button
                                                    type="button"
                                                    class={cn(
                                                        'settings-sidebar-item',
                                                        activeTab() === tab.identifier &&
                                                            'is-active'
                                                    )}
                                                    onClick={() => setActiveTab(tab.identifier)}
                                                    aria-current={
                                                        activeTab() === tab.identifier
                                                            ? 'page'
                                                            : undefined
                                                    }
                                                >
                                                    <tab.icon size={16} />
                                                    <span>{tab.label}</span>
                                                </button>
                                            </li>
                                        )}
                                    </For>
                                </ul>
                            </div>
                        )}
                    </For>
                </nav>

                {/* Panel Content */}
                <div class="settings-panel">
                    <Show when={activeTab() === 'general'}>
                        <GeneralPanel />
                    </Show>
                    <Show when={activeTab() === 'appearance'}>
                        <AppearancePanel />
                    </Show>
                    <Show when={activeTab() === 'performance'}>
                        <PerformancePanel />
                    </Show>
                    <Show when={activeTab() === 'storage'}>
                        <StoragePanel />
                    </Show>
                    <Show when={activeTab() === 'keyboard-shortcuts'}>
                        <KeyboardShortcutsPanel />
                    </Show>
                    <Show when={activeTab() === 'folders'}>
                        <FoldersPanel />
                    </Show>
                    <Show when={activeTab() === 'duplicates'}>
                        <DuplicatesPanel />
                    </Show>
                    <Show when={activeTab() === 'about'}>
                        <AboutPanel />
                    </Show>
                </div>
            </div>
        </Modal>
    );
};
