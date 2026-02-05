/**
 * Shortcut Store
 * Reactive state for registered shortcuts
 */

import { createSignal } from 'solid-js';
import { invoke } from '@tauri-apps/api/core';
import type { 
  ShortcutDefinition, 
  RegisteredShortcut, 
  ShortcutActions,
  InputScopeName,
  SerializedShortcut,
} from '../types';
import { normalizeKeysToTokens, canonicalizeShortcut } from '../normalizer';

// =============================================================================
// Default Shortcuts Definition
// =============================================================================

const DEFAULT_SHORTCUTS: ShortcutDefinition[] = [
  // Global scope
  {
    name: 'Focus Search',
    description: 'Focus the search input',
    keys: 'Meta+KeyK',
    scope: 'global',
    command: 'app:focus-search',
    category: 'Navigation',
    ignoreInputs: false, // Works even in inputs
  },
  {
    name: 'Select All',
    description: 'Select all items',
    keys: 'Meta+KeyA',
    scope: 'global',
    command: 'app:select-all',
    category: 'Selection',
  },
  {
    name: 'Deselect All',
    description: 'Clear selection',
    keys: 'Escape',
    scope: 'global',
    command: 'app:deselect-all',
    category: 'Selection',
    ignoreInputs: false, // Escape always works
  },
  {
    name: 'Settings',
    description: 'Open application settings',
    keys: 'Meta+Comma',
    scope: 'global',
    command: 'app:settings',
    category: 'Application',
  },
  {
    name: 'Clear Search / Blur',
    description: 'Clear search query or blur input',
    keys: 'Escape',
    scope: 'search',
    command: 'search:clear',
    category: 'Search',
    ignoreInputs: false,
  },
  
  // Viewport Interaction
  {
    name: 'Move Up',
    description: 'Navigate up in grid/list',
    keys: 'ArrowUp',
    scope: 'viewport',
    command: 'viewport:move-up',
    category: 'Navigation',
  },
  {
    name: 'Move Down',
    description: 'Navigate down in grid/list',
    keys: 'ArrowDown',
    scope: 'viewport',
    command: 'viewport:move-down',
    category: 'Navigation',
  },
  {
    name: 'Move Left',
    description: 'Navigate left in grid',
    keys: 'ArrowLeft',
    scope: 'viewport',
    command: 'viewport:move-left',
    category: 'Navigation',
  },
  {
    name: 'Move Right',
    description: 'Navigate right in grid',
    keys: 'ArrowRight',
    scope: 'viewport',
    command: 'viewport:move-right',
    category: 'Navigation',
  },
  {
    name: 'Go to Start',
    description: 'Navigate to first item',
    keys: 'Home',
    scope: 'viewport',
    command: 'viewport:home',
    category: 'Navigation',
  },
  {
    name: 'Go to End',
    description: 'Navigate to last item',
    keys: 'End',
    scope: 'viewport',
    command: 'viewport:end',
    category: 'Navigation',
  },
  {
    name: 'Toggle Selection',
    description: 'Select/deselect focused item',
    keys: 'Space',
    scope: 'viewport',
    command: 'viewport:toggle-select',
    category: 'Selection',
  },
  {
    name: 'Open Item',
    description: 'Open focused item',
    keys: 'Enter',
    scope: 'viewport',
    command: 'viewport:open',
    category: 'Navigation',
  },

  // Image Viewer scope
  {
    name: 'Close Viewer',
    description: 'Close the image viewer',
    keys: 'Escape',
    scope: 'image-viewer',
    command: 'viewer:close',
    category: 'Viewer',
    ignoreInputs: false,
  },
  {
    name: 'Zoom In',
    description: 'Increase zoom level',
    keys: 'Equal',
    scope: 'image-viewer',
    command: 'viewer:zoom-in',
    category: 'Viewer',
  },
  {
    name: 'Zoom Out',
    description: 'Decrease zoom level',
    keys: 'Minus',
    scope: 'image-viewer',
    command: 'viewer:zoom-out',
    category: 'Viewer',
  },
  {
    name: 'Fit to Screen',
    description: 'Fit image to screen',
    keys: 'Meta+Digit0',
    scope: 'image-viewer',
    command: 'viewer:fit-screen',
    category: 'Viewer',
  },
  {
    name: 'Original Size',
    description: 'Show image at 100% zoom',
    keys: 'Meta+Digit1',
    scope: 'image-viewer',
    command: 'viewer:original-size',
    category: 'Viewer',
  },
  {
    name: 'Pan Tool',
    description: 'Activate pan tool',
    keys: 'KeyH',
    scope: 'image-viewer',
    command: 'viewer:tool-pan',
    category: 'Viewer',
  },
  {
    name: 'Rotate Tool',
    description: 'Activate rotate tool',
    keys: 'KeyR',
    scope: 'image-viewer',
    command: 'viewer:tool-rotate',
    category: 'Viewer',
  },
  {
    name: 'Previous Item',
    description: 'Go to previous item',
    keys: 'ArrowLeft',
    scope: 'image-viewer',
    command: 'viewer:previous',
    category: 'Viewer',
  },
  {
    name: 'Next Item',
    description: 'Go to next item',
    keys: 'ArrowRight',
    scope: 'image-viewer',
    command: 'viewer:next',
    category: 'Viewer',
  },
  {
    name: 'Play/Pause Slideshow',
    description: 'Toggle slideshow playback',
    keys: 'Space',
    scope: 'image-viewer',
    command: 'viewer:slideshow-toggle',
    category: 'Viewer',
  },
  {
    name: 'Flip Horizontal',
    description: 'Flip image horizontally',
    keys: 'Shift+KeyH',
    scope: 'image-viewer',
    command: 'viewer:flip-h',
    category: 'Viewer',
  },
  {
    name: 'Flip Vertical',
    description: 'Flip image vertically',
    keys: 'Shift+KeyV',
    scope: 'image-viewer',
    command: 'viewer:flip-v',
    category: 'Viewer',
  },
  
  // Modal scope
  {
    name: 'Close Modal',
    description: 'Close the active modal',
    keys: 'Escape',
    scope: 'modal',
    command: 'modal:close',
    category: 'Modal',
    ignoreInputs: false,
  },
];

// =============================================================================
// Store Creation
// =============================================================================

function createShortcutStore() {
  const [shortcuts, setShortcuts] = createSignal<Map<string, RegisteredShortcut>>(new Map());
  const [nextId, setNextId] = createSignal(1);
  const [customizations, setCustomizations] = createSignal<Map<string, string>>(new Map());
  
  // =============================================================================
  // Internal Helpers
  // =============================================================================
  
  function generateId(): string {
    const id = `sc_${nextId()}`;
    setNextId(prev => prev + 1);
    return id;
  }
  
  function createRegisteredShortcut(
    definition: ShortcutDefinition, 
    handler?: ShortcutDefinition['handler']
  ): RegisteredShortcut {
    const id = definition.id || generateId();
    const finalHandler = handler || definition.handler;
    
    // Apply customization if exists
    const customKeys = customizations().get(id);
    const finalKeys = customKeys || (typeof definition.keys === 'string' ? definition.keys : definition.keys.join(' '));
    
    const tokens = normalizeKeysToTokens(finalKeys);
    
    return {
      ...definition,
      id,
      handler: finalHandler,
      keys: finalKeys,
      tokens,
      normalizedKeys: canonicalizeShortcut(finalKeys),
      scope: definition.scope || 'global',
      priority: definition.priority ?? 0,
      ignoreInputs: definition.ignoreInputs ?? (tokens.some(t => {
        const mods = (t.meta as any)?.modifiers;
        return Array.isArray(mods) && mods.length > 0;
      }) ? false : true),
      preventDefault: definition.preventDefault ?? true,
      isDefault: definition.isDefault ?? true,
    };
  }
  
  // =============================================================================
  // Actions
  // =============================================================================
  
  const actions: ShortcutActions = {
    register: (definition: ShortcutDefinition, handler?: ShortcutDefinition['handler']) => {
      const registered = createRegisteredShortcut(definition, handler);
      
      setShortcuts(prev => {
        const next = new Map(prev);
        next.set(registered.id, registered);
        return next;
      });
      
      return registered.id;
    },
    
    unregister: (id: string) => {
      setShortcuts(prev => {
        const next = new Map(prev);
        next.delete(id);
        return next;
      });
    },
    
    edit: (id: string, newKeys: string, persist = true) => {
      const current = shortcuts().get(id);
      if (!current) {
        console.warn(`[ShortcutStore] Cannot edit: shortcut ${id} not found`);
        return;
      }
      
      // Save customization
      setCustomizations(prev => {
        const next = new Map(prev);
        next.set(id, newKeys);
        return next;
      });
      
      // Update the registered shortcut
      const tokens = normalizeKeysToTokens(newKeys);
      const updated: RegisteredShortcut = {
        ...current,
        keys: newKeys,
        tokens,
        normalizedKeys: canonicalizeShortcut(newKeys),
        isDefault: false,
      };
      
      setShortcuts(prev => {
        const next = new Map(prev);
        next.set(id, updated);
        return next;
      });
      
      if (persist) {
        saveToBackend();
      }
    },
    
    resetToDefault: (id: string) => {
      const defaultDef = DEFAULT_SHORTCUTS.find(d => d.id === id || d.name === shortcuts().get(id)?.name);
      if (!defaultDef) {
        console.warn(`[ShortcutStore] Cannot reset: no default found for ${id}`);
        return;
      }
      
      // Remove customization
      setCustomizations(prev => {
        const next = new Map(prev);
        next.delete(id);
        return next;
      });
      
      // Re-register with default
      const registered = createRegisteredShortcut({ ...defaultDef, id });
      setShortcuts(prev => {
        const next = new Map(prev);
        next.set(id, registered);
        return next;
      });
      
      saveToBackend();
    },
    
    resetAllToDefaults: () => {
      setCustomizations(new Map());
      
      // Re-register all defaults
      const newShortcuts = new Map<string, RegisteredShortcut>();
      for (const def of DEFAULT_SHORTCUTS) {
        const registered = createRegisteredShortcut(def);
        newShortcuts.set(registered.id, registered);
      }
      setShortcuts(newShortcuts);
      saveToBackend();
    },
    
    list: () => {
      return Array.from(shortcuts().values());
    },
    
    getByScope: (scope: InputScopeName) => {
      return Array.from(shortcuts().values()).filter(s => s.scope === scope);
    },
    
    detectConflicts: (keys: string, excludeId?: string, scope?: string) => {
      const normalized = canonicalizeShortcut(keys);
      // Conflict exists if keys match AND scopes match
      const targetScope = scope || 'global';
      
      return Array.from(shortcuts().values())
        .filter(s => {
          if (s.id === excludeId) return false;
          if (s.normalizedKeys !== normalized) return false;
          // Only flag conflict if scopes are the same
          // This allows shadowing (e.g. Modal Esc vs Global Esc)
          const sScope = s.scope || 'global';
          return sScope === targetScope;
        })
        .map(s => s.name);
    },
  };
  
  // =============================================================================
  // Initialization - Run immediately
  // =============================================================================
  
  // Register default shortcuts
  for (const def of DEFAULT_SHORTCUTS) {
    actions.register(def);
  }
  
  // =============================================================================
  // Persistence Helpers
  // =============================================================================
  
  function serialize(): SerializedShortcut[] {
    return Array.from(shortcuts().values()).map(s => ({
      id: s.id,
      name: s.name,
      description: s.description,
      keys: s.keys as string,
      scope: s.scope || 'global',
      category: s.category,
      isCustom: !s.isDefault,
    }));
  }
  
  async function saveToBackend() {
    try {
      const data: Record<string, string> = {};
      const custom = customizations();
      const all = shortcuts();
      
      for (const [id, keys] of custom) {
          const s = all.get(id);
          if (s) {
              const key = `${s.name}::${s.scope || 'global'}`;
              data[key] = keys;
          }
      }
      
      await invoke('set_setting', { key: 'shortcuts', value: data });
    } catch (e) {
      console.warn('[ShortcutStore] Failed to save shortcuts:', e);
    }
  }

  async function loadFromBackend() {
    try {
      const saved = await invoke<Record<string, string> | null>('get_setting', { key: 'shortcuts' });
      if (saved) {
         for (const [key, keys] of Object.entries(saved)) {
             const [name, scope] = key.split('::');
             
             // Find by name/scope in shortcuts (defaults are already registered)
             const found = Array.from(shortcuts().values()).find(s => 
                 s.name === name && (s.scope || 'global') === (scope || 'global')
             );
             
             if (found) {
                 actions.edit(found.id, keys, false);
             }
         }
      }
    } catch (e) {
      console.warn('[ShortcutStore] Failed to load shortcuts:', e);
    }
  }
  
  // Initial Load from Backend
  loadFromBackend();
  
  // =============================================================================
  // Getters
  // =============================================================================
  
  function getById(id: string): RegisteredShortcut | undefined {
    return shortcuts().get(id);
  }
  
  function getByCommand(command: string): RegisteredShortcut | undefined {
    return Array.from(shortcuts().values()).find(s => s.command === command);
  }
  
  function getCategories(): string[] {
    const cats = new Set<string>();
    for (const s of shortcuts().values()) {
      if (s.category) cats.add(s.category);
    }
    return Array.from(cats).sort();
  }

  function getDefault(id: string): ShortcutDefinition | undefined {
    // Try to find by ID first, then by name match if ID was generated
    const current = shortcuts().get(id);
    if (!current) return undefined;
    
    return DEFAULT_SHORTCUTS.find(d => 
      d.id === id || (current && d.name === current.name)
    );
  }

  function getByNameAndScope(name: string, scope: InputScopeName = 'global'): RegisteredShortcut | undefined {
    return Array.from(shortcuts().values()).find(s => 
      s.name === name && (s.scope || 'global') === (scope || 'global')
    );
  }
  
  return {
    // State
    shortcuts,
    customizations,
    
    // Actions
    ...actions,
    
    // Getters
    getById,
    getByCommand,
    getByNameAndScope,
    getCategories,
    
    // Persistence
    serialize,
    getDefault,
  };
}

// =============================================================================
// Singleton Export
// =============================================================================

// Create store directly - signals work outside component context
export const shortcutStore = createShortcutStore();
