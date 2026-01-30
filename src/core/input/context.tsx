/**
 * Input Context & Provider
 * Initializes the input system and provides access to stores
 */

import { 
  Component, 
  JSX, 
  createContext, 
  useContext, 
  onMount,
  onCleanup,
} from 'solid-js';
import { inputStore } from './store/inputStore';
import { shortcutStore } from './store/shortcutStore';
import { createKeyboardProvider } from './providers/KeyboardProvider';
import { createPointerProvider } from './providers/PointerProvider';
import { createGestureProvider } from './providers/GestureProvider';
import { clearCommandHandlers } from './dispatcher';
import type { InputProviderConfig } from './types';

// =============================================================================
// Context Definition
// =============================================================================

interface InputContextValue {
  // Input state accessors
  enabled: typeof inputStore.enabled;
  pressedKeys: typeof inputStore.pressedKeys;
  activeScopes: typeof inputStore.activeScopes;
  currentScope: typeof inputStore.currentScope;
  isKeyPressed: typeof inputStore.isKeyPressed;
  
  // Input actions
  enable: typeof inputStore.enable;
  disable: typeof inputStore.disable;
  pushScope: typeof inputStore.pushScope;
  popScope: typeof inputStore.popScope;
  
  // Shortcut accessors
  shortcuts: typeof shortcutStore.shortcuts;
  listShortcuts: typeof shortcutStore.list;
  getShortcutsByScope: typeof shortcutStore.getByScope;
  getShortcutCategories: typeof shortcutStore.getCategories;
  
  // Shortcut actions
  registerShortcut: typeof shortcutStore.register;
  unregisterShortcut: typeof shortcutStore.unregister;
  editShortcut: typeof shortcutStore.edit;
  resetShortcut: typeof shortcutStore.resetToDefault;
  resetAllShortcuts: typeof shortcutStore.resetAllToDefaults;
  getConflicts: typeof shortcutStore.getConflicts;
}

const InputContext = createContext<InputContextValue>();

// =============================================================================
// Provider Component
// =============================================================================

interface InputProviderProps {
  children: JSX.Element;
  config?: InputProviderConfig;
}

/**
 * Input Provider Component
 * Wrap your app with this to enable the input system
 */
export const InputProvider: Component<InputProviderProps> = (props) => {
  // Store cleanup functions
  let keyboardCleanup: (() => void) | null = null;
  
  // Initialize providers on mount
  onMount(() => {
    // Initialize shortcut store with defaults
    shortcutStore.initialize();
    
    // Start keyboard provider
    keyboardCleanup = createKeyboardProvider();
    
    // TODO: Add PointerProvider and GestureProvider when ready
    // pointerCleanup = createPointerProvider();
    // gestureCleanup = createGestureProvider();
  });
  
  // Cleanup on unmount
  onCleanup(() => {
    if (keyboardCleanup) {
      keyboardCleanup();
      keyboardCleanup = null;
    }
    clearCommandHandlers();
  });
  
  const contextValue: InputContextValue = {
    // Input state
    enabled: inputStore.enabled,
    pressedKeys: inputStore.pressedKeys,
    activeScopes: inputStore.activeScopes,
    currentScope: inputStore.currentScope,
    isKeyPressed: inputStore.isKeyPressed,
    
    // Input actions
    enable: inputStore.enable,
    disable: inputStore.disable,
    pushScope: inputStore.pushScope,
    popScope: inputStore.popScope,
    
    // Shortcut state
    shortcuts: shortcutStore.shortcuts,
    listShortcuts: shortcutStore.list,
    getShortcutsByScope: shortcutStore.getByScope,
    getShortcutCategories: shortcutStore.getCategories,
    
    // Shortcut actions
    registerShortcut: shortcutStore.register,
    unregisterShortcut: shortcutStore.unregister,
    editShortcut: shortcutStore.edit,
    resetShortcut: shortcutStore.resetToDefault,
    resetAllShortcuts: shortcutStore.resetAllToDefaults,
    getConflicts: shortcutStore.getConflicts,
  };
  
  return (
    <InputContext.Provider value={contextValue}>
      {props.children}
    </InputContext.Provider>
  );
};

// =============================================================================
// Hook
// =============================================================================

/**
 * Access the input context
 * Must be used within InputProvider
 * 
 * @example
 * const input = useInput();
 * 
 * // Check if a key is pressed
 * const isShiftDown = () => input.isKeyPressed('Shift');
 * 
 * // List all shortcuts
 * const shortcuts = input.listShortcuts();
 */
export function useInput(): InputContextValue {
  const context = useContext(InputContext);
  
  if (!context) {
    throw new Error('useInput must be used within InputProvider');
  }
  
  return context;
}

/**
 * Optional: Get input context without throwing
 * Returns undefined if not in provider
 */
export function useInputOptional(): InputContextValue | undefined {
  return useContext(InputContext);
}
