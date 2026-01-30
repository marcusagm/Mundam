/**
 * Input Service
 * Public API exports for the input/shortcut system
 */

// =============================================================================
// Types
// =============================================================================

export type {
  // Platform & Modifiers
  Platform,
  ModifierKey,
  ModifierState,
  
  // Scopes
  InputScopeName,
  InputScope,
  
  // Tokens
  TokenKind,
  InputToken,
  KeyboardToken,
  PointerToken,
  WheelToken,
  GestureToken,
  
  // Gestures
  GestureType,
  SwipeDirection,
  GesturePayload,
  
  // Shortcuts
  ShortcutDefinition,
  RegisteredShortcut,
  ShortcutPayload,
  
  // State
  InputState,
  ShortcutState,
  
  // Options
  CreateShortcutOptions,
  CreateGestureOptions,
  CreateInputScopeOptions,
  InputProviderConfig,
  
  // Serialization
  SerializedShortcut,
  ShortcutPreferences,
} from './types';

export { SCOPE_PRIORITIES } from './types';

// =============================================================================
// Context & Provider
// =============================================================================

export { InputProvider, useInput, useInputOptional } from './context';

// =============================================================================
// Primitives
// =============================================================================

export { 
  createShortcut, 
  useShortcut, 
  useShortcuts 
} from './primitives/createShortcut';

export { 
  createKeyState, 
  createPressedKeys,
  createAnyKeyPressed,
  createAllKeysPressed,
} from './primitives/createKeyState';

export { 
  createInputScope,
  createConditionalScope,
  useScopeOnMount,
} from './primitives/createInputScope';

export {
  createGesture,
  usePinchGesture,
  useSwipeGesture,
  useRotateGesture,
} from './primitives/createGesture';

// =============================================================================
// Utilities
// =============================================================================

export {
  // Platform detection
  detectPlatform,
  isMac,
  
  // Normalization
  canonicalizeShortcut,
  parseShortcutString,
  normalizeKeyCode,
  
  // Display formatting
  formatShortcutForDisplay,
  getShortcutDisplayParts,
} from './normalizer';

// =============================================================================
// Command System
// =============================================================================

export { onCommand } from './dispatcher';

// =============================================================================
// Stores (for advanced usage)
// =============================================================================

export { inputStore } from './store/inputStore';
export { shortcutStore } from './store/shortcutStore';
