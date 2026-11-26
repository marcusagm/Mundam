import { Component, createEffect, For, Show, onCleanup } from 'solid-js';
import { RotateCcw, AlertTriangle, Check, X } from 'lucide-solid';
import { shortcutStore } from '../../../../core/input/store/shortcutStore';
import {
    formatShortcutForDisplay,
    buildCanonicalId,
    resolveKeyboardEventKey
} from '../../../../core/input/normalizer';
import type { RegisteredShortcut, ModifierKey } from '../../../../core/input/types';
import { Kbd } from '../../../ui/Kbd';

function isTerminalEnter(e: KeyboardEvent): boolean {
    return e.key === 'Enter' && !e.metaKey && !e.ctrlKey && !e.altKey && !e.shiftKey;
}

function extractModifiers(e: KeyboardEvent): ModifierKey[] {
    const modifiers: ModifierKey[] = [];
    if (e.metaKey) modifiers.push('Meta');
    if (e.ctrlKey) modifiers.push('Ctrl');
    if (e.altKey) modifiers.push('Alt');
    if (e.shiftKey) modifiers.push('Shift');
    return modifiers;
}

interface ShortcutRowProps {
    shortcut: RegisteredShortcut;
    isEditing: boolean;
    recordedKeys: string | null;
    conflicts: string[];
    onStartEdit: () => void;
    onCancel: () => void;
    onSave: () => void;
    onRecordKeys: (keys: string) => void;
    onReset: () => void;
    onUseSuggestion: (keys: string) => void;
}

/** Renders a shortcut binding and its keyboard recording controls. */
export const ShortcutRow: Component<ShortcutRowProps> = props => {
    let inputRef: HTMLDivElement | undefined;

    const displayKeys = () => {
        if (props.isEditing && props.recordedKeys) {
            return props.recordedKeys;
        }
        return props.shortcut.keys;
    };

    const isModified = () => {
        // Check if shortcut has been modified from default
        const defaultShortcut = shortcutStore.getDefault(props.shortcut.id);
        if (!defaultShortcut) return false;
        return props.shortcut.keys !== defaultShortcut.keys;
    };

    const bindingConflicts = () =>
        props.shortcut.preferenceId && props.shortcut.keys
            ? shortcutStore.detectConflicts(
                  props.shortcut.normalizedKeys,
                  props.shortcut.id,
                  props.shortcut.scope
              )
            : [];

    // Handle key recording when editing
    createEffect(() => {
        if (!props.isEditing || !inputRef) return;

        inputRef.focus();

        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.key === 'Tab') return;

            e.preventDefault();
            e.stopPropagation();

            // Ignore standalone modifier keys
            if (['Meta', 'Control', 'Alt', 'Shift'].includes(e.key)) {
                return;
            }

            // Handle Enter to save
            if (isTerminalEnter(e)) {
                props.onSave();
                return;
            }

            // Build the key combination
            const modifiers = extractModifiers(e);
            const key = resolveKeyboardEventKey(e);
            const canonical = buildCanonicalId(modifiers, key);

            props.onRecordKeys(canonical);
        };

        inputRef.addEventListener('keydown', handleKeyDown);

        onCleanup(() => {
            inputRef?.removeEventListener('keydown', handleKeyDown);
        });
    });

    return (
        <div class={`shortcut-row ${props.isEditing ? 'is-editing' : ''}`}>
            <div class="shortcut-info">
                <span class="shortcut-name">{props.shortcut.name}</span>
                <Show when={props.shortcut.description}>
                    <span class="shortcut-description">{props.shortcut.description}</span>
                </Show>
                <Show when={!props.isEditing && bindingConflicts().length > 0}>
                    <span class="shortcut-conflict-warning">
                        <AlertTriangle size={14} />
                        <span>
                            Shortcut inactive. Conflicts with {bindingConflicts().join(', ')}.
                        </span>
                    </span>
                </Show>
            </div>

            <div class="shortcut-keys-container">
                <Show when={!props.isEditing}>
                    <button
                        type="button"
                        class="shortcut-keys-button"
                        onClick={() => props.onStartEdit()}
                        title="Click to edit"
                    >
                        <Show when={displayKeys()} fallback={<span>Unassigned</span>}>
                            <ShortcutKeys keys={displayKeys()} />
                        </Show>
                        <Show when={isModified()}>
                            <span class="shortcut-modified-badge" title="Modified">
                                •
                            </span>
                        </Show>
                    </button>
                    <Show when={!props.shortcut.keys && props.shortcut.suggestedKeys} keyed>
                        {suggestion => (
                            <button
                                type="button"
                                class="shortcut-keys-button"
                                onClick={() => props.onUseSuggestion(suggestion)}
                            >
                                Use suggested <ShortcutKeys keys={suggestion} />
                            </button>
                        )}
                    </Show>
                </Show>

                <Show when={props.isEditing}>
                    <div class="shortcut-edit-container">
                        <div class="shortcut-edit-row">
                            <div ref={inputRef} class="shortcut-record-input" tabindex={0}>
                                <Show
                                    when={props.recordedKeys}
                                    fallback={
                                        <span class="shortcut-record-prompt">Press keys...</span>
                                    }
                                >
                                    <ShortcutKeys keys={props.recordedKeys!} />
                                </Show>
                            </div>

                            <div class="shortcut-edit-actions">
                                <button
                                    type="button"
                                    class="shortcut-action-btn save"
                                    onClick={() => props.onSave()}
                                    disabled={!props.recordedKeys || props.conflicts.length > 0}
                                    title="Save"
                                >
                                    <Check size={14} />
                                </button>
                                <button
                                    type="button"
                                    class="shortcut-action-btn cancel"
                                    onClick={() => props.onCancel()}
                                    title="Cancel"
                                >
                                    <X size={14} />
                                </button>
                            </div>
                        </div>

                        <Show when={props.conflicts.length > 0}>
                            <div class="shortcut-conflict-warning">
                                <AlertTriangle size={14} />
                                <span>Conflicts with: {props.conflicts.join(', ')}</span>
                            </div>
                        </Show>
                    </div>
                </Show>

                <Show when={!props.isEditing && isModified()}>
                    <button
                        type="button"
                        class="shortcut-reset-btn"
                        onClick={() => props.onReset()}
                        title="Reset to default"
                    >
                        <RotateCcw size={12} />
                    </button>
                </Show>
            </div>
        </div>
    );
};

interface ShortcutKeysProps {
    keys: string | string[];
}

const ShortcutKeys: Component<ShortcutKeysProps> = props => {
    const parts = () => {
        const keysArray = Array.isArray(props.keys) ? props.keys : [props.keys];
        return keysArray.map(key => formatShortcutForDisplay(key));
    };

    return (
        <span class="shortcut-keys">
            <For each={parts()}>
                {(part, i) => (
                    <>
                        {i() > 0 && <span class="shortcut-separator">/</span>}
                        <Kbd>{part}</Kbd>
                    </>
                )}
            </For>
        </span>
    );
};
