/**
 * Shortcut Store
 * Reactive state for registered shortcuts
 */

import { createSignal } from 'solid-js';
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
    name: 'Previous Image',
    description: 'Go to previous image',
    keys: 'ArrowLeft',
    scope: 'image-viewer',
    command: 'viewer:previous',
    category: 'Viewer',
  },
  {
    name: 'Next Image',
    description: 'Go to next image',
    keys: 'ArrowRight',
    scope: 'image-viewer',
    command: 'viewer:next',
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
  const [initialized, setInitialized] = createSignal(false);
  
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
      ignoreInputs: definition.ignoreInputs ?? true,
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
    
    edit: (id: string, newKeys: string) => {
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
    },
    
    list: () => {
      return Array.from(shortcuts().values());
    },
    
    getByScope: (scope: InputScopeName) => {
      return Array.from(shortcuts().values()).filter(s => s.scope === scope);
    },
    
    getConflicts: (keys: string, excludeId?: string) => {
      const normalized = canonicalizeShortcut(keys);
      return Array.from(shortcuts().values()).filter(s => 
        s.normalizedKeys === normalized && s.id !== excludeId
      );
    },
  };
  
  // =============================================================================
  // Initialization
  // =============================================================================
  
  function initialize() {
    if (initialized()) return;
    
    // Register default shortcuts
    for (const def of DEFAULT_SHORTCUTS) {
      actions.register(def);
    }
    
    setInitialized(true);
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
  
  function loadCustomizations(customs: Map<string, string>) {
    setCustomizations(customs);
    
    // Re-apply to existing shortcuts
    for (const [id, keys] of customs) {
      const current = shortcuts().get(id);
      if (current) {
        actions.edit(id, keys);
      }
    }
  }
  
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
  
  return {
    // State
    shortcuts,
    customizations,
    initialized,
    
    // Actions
    ...actions,
    
    // Getters
    getById,
    getByCommand,
    getCategories,
    
    // Lifecycle
    initialize,
    
    // Persistence
    serialize,
    loadCustomizations,
  };
}

// =============================================================================
// Singleton Export
// =============================================================================

// Create store directly - signals work outside component context
export const shortcutStore = createShortcutStore();
