/**
 * Input Service Types
 * Complete type definitions for the reactive input/shortcut system
 */

// =============================================================================
// Platform & Modifiers
// =============================================================================

export type Platform = 'mac' | 'windows' | 'linux';

export type ModifierKey = 'Meta' | 'Ctrl' | 'Alt' | 'Shift';

export interface ModifierState {
  meta: boolean;
  ctrl: boolean;
  alt: boolean;
  shift: boolean;
}

// =============================================================================
// Scopes
// =============================================================================

export type InputScopeName = 'global' | 'image-viewer' | 'search' | 'modal' | string;

export interface InputScope {
  name: InputScopeName;
  priority: number;
  blockLowerScopes?: boolean;
}

export const SCOPE_PRIORITIES: Record<string, number> = {
  global: 0,
  viewport: 10,
  'image-viewer': 50,
  editing: 1000,
  search: 1100,
  modal: 1200,
};

// =============================================================================
// Tokens
// =============================================================================

export type TokenKind = 'keyboard' | 'pointer' | 'wheel' | 'gesture';

export interface InputToken {
  /** Type of input */
  kind: TokenKind;
  /** Canonical identifier (e.g., "Meta+KeyS") */
  id: string;
  /** Original raw input string */
  raw: string;
  /** Additional metadata */
  meta?: Record<string, unknown>;
}

export interface KeyboardToken extends InputToken {
  kind: 'keyboard';
  meta: {
    key: string;
    code: string;
    modifiers: ModifierKey[];
  };
}

export interface PointerToken extends InputToken {
  kind: 'pointer';
  meta: {
    button: number;
    pointerType: 'mouse' | 'pen' | 'touch';
    modifiers: ModifierKey[];
  };
}

export interface WheelToken extends InputToken {
  kind: 'wheel';
  meta: {
    deltaY: number;
    modifiers: ModifierKey[];
  };
}

export interface GestureToken extends InputToken {
  kind: 'gesture';
  meta: {
    gesture: GestureType;
    fingers?: number;
    direction?: SwipeDirection;
    scale?: number;
    angle?: number;
  };
}

// =============================================================================
// Gestures
// =============================================================================

export type GestureType = 'swipe' | 'pinch' | 'rotate';
export type SwipeDirection = 'up' | 'down' | 'left' | 'right';

export interface GesturePayload {
  gesture: GestureType;
  meta: {
    fingers?: number;
    direction?: SwipeDirection;
    dx?: number;
    dy?: number;
    scale?: number;
    angle?: number;
    center?: { x: number; y: number };
    final?: boolean;
    incremental?: boolean;
    duration?: number;
  };
  event: TouchEvent | null;
}

// =============================================================================
// Shortcut Definitions
// =============================================================================

export interface ShortcutDefinition {
  /** Unique identifier (auto-generated if not provided) */
  id?: string;
  /** Human-readable name for UI */
  name: string;
  /** Description for settings panel */
  description?: string;
  /** Key combination(s) - string or array for sequences */
  keys: string | string[];
  /** Scope where this shortcut is active */
  scope?: InputScopeName;
  /** Priority within scope (higher = first) */
  priority?: number;
  /** Command name for event dispatch */
  command?: string;
  /** Handler function */
  handler?: (event: Event | null, payload: ShortcutPayload) => void;
  /** Whether to prevent default browser behavior */
  preventDefault?: boolean;
  /** Whether to ignore when focus is in input/textarea (default: true) */
  ignoreInputs?: boolean;
  /** Condition function - must return true for shortcut to fire */
  enabledWhen?: () => boolean;
  /** Category for grouping in settings UI */
  category?: string;
  /** Whether this is a default shortcut (not user-customized) */
  isDefault?: boolean;
}

export interface RegisteredShortcut extends ShortcutDefinition {
  id: string;
  tokens: InputToken[];
  normalizedKeys: string;
}

export interface ShortcutPayload {
  shortcutDef: RegisteredShortcut;
  sequence: InputToken[];
  meta: Record<string, unknown>;
}

// =============================================================================
// Store State
// =============================================================================

export interface InputState {
  /** Whether the input service is active */
  enabled: boolean;
  /** Currently pressed keys (by canonical ID) */
  pressedKeys: Set<string>;
  /** Stack of active scopes (last is highest priority) */
  scopeStack: InputScope[];
  /** Sequence buffer for multi-key shortcuts */
  sequenceBuffer: InputToken[];
  /** Shortcuts that have been dispatched (for chord deduplication) */
  dispatchedChords: Set<string>;
}

export interface ShortcutState {
  /** All registered shortcuts */
  shortcuts: Map<string, RegisteredShortcut>;
  /** Counter for generating IDs */
  nextId: number;
  /** User customizations (keyed by original shortcut ID) */
  customizations: Map<string, string>; // originalId -> customKeys
}

// =============================================================================
// Actions
// =============================================================================

export interface InputActions {
  enable: () => void;
  disable: () => void;
  pushScope: (name: InputScopeName, priority?: number, blockLowerScopes?: boolean) => void;
  popScope: (name: InputScopeName) => void;
  keyDown: (token: InputToken) => void;
  keyUp: (keyId: string) => void;
  clearPressed: () => void;
}

export interface ShortcutActions {
  register: (definition: ShortcutDefinition, handler?: ShortcutDefinition['handler']) => string;
  unregister: (id: string) => void;
  edit: (id: string, newKeys: string, persist?: boolean) => void;
  resetToDefault: (id: string) => void;
  resetAllToDefaults: () => void;
  list: () => RegisteredShortcut[];
  getByScope: (scope: InputScopeName) => RegisteredShortcut[];
  detectConflicts: (keys: string, excludeId?: string, scope?: string) => string[];
}

// =============================================================================
// Provider Config
// =============================================================================

export interface InputProviderConfig {
  /** Timeout for key sequences in ms (default: 800) */
  sequenceTimeout?: number;
  /** Throttle for gestures in ms (default: 50) */
  gestureThrottleMs?: number;
  /** Whether to enable gesture recognition (default: true) */
  enableGestures?: boolean;
}

// =============================================================================
// Primitive Options
// =============================================================================

export interface CreateShortcutOptions {
  keys: string | string[];
  action: (event: Event | null, payload: ShortcutPayload) => void;
  name?: string;
  description?: string;
  scope?: InputScopeName;
  priority?: number;
  preventDefault?: boolean;
  ignoreInputs?: boolean;
  enabled?: () => boolean;
  category?: string;
}

export interface CreateGestureOptions {
  type: GestureType;
  handler: (payload: GesturePayload) => void;
  scope?: InputScopeName;
  enabled?: () => boolean;
}

export interface CreateInputScopeOptions {
  name: InputScopeName;
  priority?: number;
  blockLowerScopes?: boolean;
}

// =============================================================================
// Serialization (for backend persistence)
// =============================================================================

export interface SerializedShortcut {
  id: string;
  name: string;
  description?: string;
  keys: string;
  scope: string;
  category?: string;
  isCustom: boolean;
}

export interface ShortcutPreferences {
  shortcuts: SerializedShortcut[];
  version: number;
}
