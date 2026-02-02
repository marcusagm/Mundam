/**
 * Keyboard Shortcuts Panel
 * Settings panel for viewing and editing keyboard shortcuts
 */

import { 
  Component, 
  createSignal, 
  createMemo,
  For, 
  Show,
  onCleanup,
  createEffect 
} from 'solid-js';
import { RotateCcw, AlertTriangle, Check, X } from 'lucide-solid';
import { shortcutStore } from '../../../core/input/store/shortcutStore';
import { formatShortcutForDisplay, buildCanonicalId } from '../../../core/input/normalizer';
import type { RegisteredShortcut, ModifierKey } from '../../../core/input/types';
import { Button } from '../../ui/Button';
import { Kbd } from '../../ui/Kbd';
import './keyboard-shortcuts-panel.css';

type ScopeGroup = {
  scope: string;
  label: string;
  shortcuts: RegisteredShortcut[];
};

const SCOPE_LABELS: Record<string, string> = {
  global: 'Global',
  'image-viewer': 'Image Viewer',
  viewport: 'Viewport',
  search: 'Search',
  modal: 'Modal',
};

export const KeyboardShortcutsPanel: Component = () => {
  const [editingId, setEditingId] = createSignal<string | null>(null);
  const [recordedKeys, setRecordedKeys] = createSignal<string | null>(null);
  const [conflicts, setConflicts] = createSignal<string[]>([]);
  
  // Group shortcuts by scope
  const groupedShortcuts = createMemo((): ScopeGroup[] => {
    const shortcuts = shortcutStore.list();
    const groups = new Map<string, RegisteredShortcut[]>();
    
    for (const shortcut of shortcuts) {
      const scope = shortcut.scope || 'global';
      if (!groups.has(scope)) {
        groups.set(scope, []);
      }
      groups.get(scope)!.push(shortcut);
    }
    
    // Sort groups by priority
    const order = ['global', 'image-viewer', 'viewport', 'search', 'modal'];
    const result: ScopeGroup[] = [];
    
    for (const scope of order) {
      const shortcuts = groups.get(scope);
      if (shortcuts && shortcuts.length > 0) {
        result.push({
          scope,
          label: SCOPE_LABELS[scope] || scope,
          shortcuts: shortcuts.sort((a, b) => a.name.localeCompare(b.name)),
        });
      }
    }
    
    // Add any remaining scopes
    for (const [scope, shortcuts] of groups) {
      if (!order.includes(scope) && shortcuts.length > 0) {
        result.push({
          scope,
          label: SCOPE_LABELS[scope] || scope,
          shortcuts: shortcuts.sort((a, b) => a.name.localeCompare(b.name)),
        });
      }
    }
    
    return result;
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
  
  const saveShortcut = () => {
    const id = editingId();
    const keys = recordedKeys();
    
    if (!id || !keys) return;
    
    // Check for conflicts
    const shortcut = shortcutStore.getById(id);
    const conflictList = shortcutStore.detectConflicts(keys, id, shortcut?.scope);
    
    if (conflictList.length > 0) {
      setConflicts(conflictList);
      return;
    }
    
    shortcutStore.edit(id, keys);
    cancelEditing();
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
          {(group) => (
            <section class="shortcuts-group">
              <h3 class="shortcuts-group-title">{group.label}</h3>
              <div class="shortcuts-list">
                <For each={group.shortcuts}>
                  {(shortcut) => (
                    <ShortcutRow
                      shortcut={shortcut}
                      isEditing={editingId() === shortcut.id}
                      recordedKeys={editingId() === shortcut.id ? recordedKeys() : null}
                      conflicts={editingId() === shortcut.id ? conflicts() : []}
                      onStartEdit={() => startEditing(shortcut.id)}
                      onCancel={cancelEditing}
                      onSave={saveShortcut}
                      onRecordKeys={setRecordedKeys}
                      onReset={() => resetToDefault(shortcut.id)}
                    />
                  )}
                </For>
              </div>
            </section>
          )}
        </For>
      </div>
    </div>
  );
};

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
}

const ShortcutRow: Component<ShortcutRowProps> = (props) => {
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
  
  // Handle key recording when editing
  createEffect(() => {
    if (!props.isEditing || !inputRef) return;
    
    inputRef.focus();
    
    const handleKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      
      // Ignore standalone modifier keys
      if (['Meta', 'Control', 'Alt', 'Shift'].includes(e.key)) {
        return;
      }
      
      // Handle Enter to save
      if (e.key === 'Enter' && !e.metaKey && !e.ctrlKey && !e.altKey && !e.shiftKey) {
        props.onSave();
        return;
      }
      
      // Build the key combination
      const modifiers: ModifierKey[] = [];
      if (e.metaKey) modifiers.push('Meta');
      if (e.ctrlKey) modifiers.push('Ctrl');
      if (e.altKey) modifiers.push('Alt');
      if (e.shiftKey) modifiers.push('Shift');
      
      const key = e.code || e.key;
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
      </div>
      
      <div class="shortcut-keys-container">
        <Show when={!props.isEditing}>
          <button
            type="button"
            class="shortcut-keys-button"
            onClick={props.onStartEdit}
            title="Click to edit"
          >
            <ShortcutKeys keys={displayKeys()} />
            <Show when={isModified()}>
              <span class="shortcut-modified-badge" title="Modified">•</span>
            </Show>
          </button>
        </Show>
        
        <Show when={props.isEditing}>
          <div class="shortcut-edit-container">
            <div class="shortcut-edit-row">
              <div
                ref={inputRef}
                class="shortcut-record-input"
                tabindex={0}
              >
                <Show 
                  when={props.recordedKeys} 
                  fallback={<span class="shortcut-record-prompt">Press keys...</span>}
                >
                  <ShortcutKeys keys={props.recordedKeys!} />
                </Show>
              </div>
              
              <div class="shortcut-edit-actions">
                <button
                  type="button"
                  class="shortcut-action-btn save"
                  onClick={props.onSave}
                  disabled={!props.recordedKeys || props.conflicts.length > 0}
                  title="Save"
                >
                  <Check size={14} />
                </button>
                <button
                  type="button"
                  class="shortcut-action-btn cancel"
                  onClick={props.onCancel}
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
            onClick={props.onReset}
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

const ShortcutKeys: Component<ShortcutKeysProps> = (props) => {
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
