/**
 * Keyboard Shortcuts Panel
 * Settings panel for viewing and editing keyboard shortcuts
 */

import { Component, createSignal, createMemo, For } from 'solid-js';
import { RotateCcw } from 'lucide-solid';
import { shortcutStore } from '../../../core/input/store/shortcutStore';
import { SectionGroup } from '../../ui/SectionGroup';
import { MouseInteractionsReference } from './MouseInteractionsReference';
import { ShortcutRow } from './keyboard/ShortcutRow';
import './keyboard-shortcuts-panel.css';
import { Button } from '../../ui/Button';
import {
    buildOrderedScopeGroups,
    groupShortcutsByScope,
    type ScopeGroup
} from './data/shortcutScopeGroups';

export const KeyboardShortcutsPanel: Component = () => {
    const [editingId, setEditingId] = createSignal<string | null>(null);
    const [recordedKeys, setRecordedKeys] = createSignal<string | null>(null);
    const [conflicts, setConflicts] = createSignal<string[]>([]);

    // Group shortcuts by scope
    const groupedShortcuts = createMemo((): ScopeGroup[] => {
        // Filter out internal system shortcuts (like UI component navigation)
        const shortcuts = shortcutStore.list().filter(s => !s.system);
        const groups = groupShortcutsByScope(shortcuts);
        return buildOrderedScopeGroups(groups);
    });

    const startEditing = (shortcutId: string) => {
        setEditingId(shortcutId);
        setRecordedKeys(null);
        setConflicts([]);
    };

    const cancelEditing = () => {
        setEditingId(null);
        setRecordedKeys(null);
        setConflicts([]);
    };

    const applyShortcut = (id: string, keys: string) => {
        const shortcut = shortcutStore.getById(id);
        if (!shortcut) return;
        const conflictList = shortcutStore.detectConflicts(keys, id, shortcut.scope);

        if (conflictList.length > 0) {
            setEditingId(id);
            setRecordedKeys(keys);
            setConflicts(conflictList);
            return;
        }

        shortcutStore.edit(id, keys);
        cancelEditing();
    };

    const saveShortcut = () => {
        const id = editingId();
        const keys = recordedKeys();
        if (id && keys) applyShortcut(id, keys);
    };

    const recordShortcut = (keys: string) => {
        setRecordedKeys(keys);
        setConflicts([]);
    };

    const resetToDefault = (shortcutId: string) => {
        shortcutStore.resetToDefault(shortcutId);
    };

    const resetAllToDefaults = () => {
        shortcutStore.resetAllToDefaults();
    };

    return (
        <div class="settings-panel-content shortcuts-panel">
            <div class="shortcuts-panel-header">
                <div>
                    <h2 class="settings-panel-title">Keyboard Shortcuts</h2>
                    <p class="settings-panel-description">
                        Customize keyboard shortcuts for various actions.
                    </p>
                </div>
                <Button
                    variant="secondary"
                    size="sm"
                    onClick={resetAllToDefaults}
                    leftIcon={<RotateCcw />}
                >
                    Reset All
                </Button>
            </div>

            <div class="shortcuts-groups">
                <For each={groupedShortcuts()}>
                    {group => (
                        <SectionGroup title={group.label}>
                            <For each={group.shortcuts}>
                                {shortcut => (
                                    <ShortcutRow
                                        shortcut={shortcut}
                                        isEditing={editingId() === shortcut.id}
                                        recordedKeys={
                                            editingId() === shortcut.id ? recordedKeys() : null
                                        }
                                        conflicts={editingId() === shortcut.id ? conflicts() : []}
                                        onStartEdit={() => startEditing(shortcut.id)}
                                        onCancel={cancelEditing}
                                        onSave={saveShortcut}
                                        onRecordKeys={recordShortcut}
                                        onReset={() => resetToDefault(shortcut.id)}
                                        onUseSuggestion={keys => applyShortcut(shortcut.id, keys)}
                                    />
                                )}
                            </For>
                        </SectionGroup>
                    )}
                </For>
            </div>

            <MouseInteractionsReference />
        </div>
    );
};
