/**
 * Token Normalizer
 * Pure utility functions for normalizing and comparing input tokens.
 * Platform-aware (Mac uses Meta, Windows/Linux use Ctrl for "primary" modifier)
 */

import type { 
  InputToken, 
  ModifierKey, 
  Platform,
  TokenKind 
} from './types';

// =============================================================================
// Platform Detection
// =============================================================================

let cachedPlatform: Platform | null = null;

export function detectPlatform(): Platform {
  if (cachedPlatform) return cachedPlatform;
  
  if (typeof navigator === 'undefined') {
    cachedPlatform = 'windows';
    return cachedPlatform;
  }
  
  const ua = navigator.userAgent.toLowerCase();
  
  if (ua.includes('mac')) {
    cachedPlatform = 'mac';
  } else if (ua.includes('linux')) {
    cachedPlatform = 'linux';
  } else {
    cachedPlatform = 'windows';
  }
  
  return cachedPlatform;
}

export function isMac(): boolean {
  return detectPlatform() === 'mac';
}

// =============================================================================
// Modifier Helpers
// =============================================================================

const MODIFIER_ORDER: ModifierKey[] = ['Meta', 'Ctrl', 'Alt', 'Shift'];

const MODIFIER_ALIASES: Record<string, ModifierKey> = {
  cmd: 'Meta',
  command: 'Meta',
  meta: 'Meta',
  ctrl: 'Ctrl',
  control: 'Ctrl',
  alt: 'Alt',
  option: 'Alt',
  shift: 'Shift',
};

export function normalizeModifier(mod: string): ModifierKey | null {
  const lower = mod.toLowerCase();
  return MODIFIER_ALIASES[lower] || null;
}

export function sortModifiers(mods: ModifierKey[]): ModifierKey[] {
  return [...mods].sort((a, b) => 
    MODIFIER_ORDER.indexOf(a) - MODIFIER_ORDER.indexOf(b)
  );
}

export function extractModifiersFromEvent(event: KeyboardEvent | MouseEvent | WheelEvent): ModifierKey[] {
  const mods: ModifierKey[] = [];
  if (event.metaKey) mods.push('Meta');
  if (event.ctrlKey) mods.push('Ctrl');
  if (event.altKey) mods.push('Alt');
  if (event.shiftKey) mods.push('Shift');
  return mods;
}

// =============================================================================
// Key Normalization
// =============================================================================

const KEY_CODE_MAP: Record<string, string> = {
  // Letters are already in correct format (KeyA, KeyB, etc.)
  // Numbers
  '0': 'Digit0', '1': 'Digit1', '2': 'Digit2', '3': 'Digit3', '4': 'Digit4',
  '5': 'Digit5', '6': 'Digit6', '7': 'Digit7', '8': 'Digit8', '9': 'Digit9',
  // Special keys
  'escape': 'Escape',
  'esc': 'Escape',
  'enter': 'Enter',
  'return': 'Enter',
  'tab': 'Tab',
  'space': 'Space',
  ' ': 'Space',
  'backspace': 'Backspace',
  'delete': 'Delete',
  'del': 'Delete',
  'insert': 'Insert',
  'ins': 'Insert',
  'home': 'Home',
  'end': 'End',
  'pageup': 'PageUp',
  'pagedown': 'PageDown',
  'pgup': 'PageUp',
  'pgdn': 'PageDown',
  // Arrow keys
  'arrowup': 'ArrowUp',
  'arrowdown': 'ArrowDown',
  'arrowleft': 'ArrowLeft',
  'arrowright': 'ArrowRight',
  'up': 'ArrowUp',
  'down': 'ArrowDown',
  'left': 'ArrowLeft',
  'right': 'ArrowRight',
  // Function keys
  'f1': 'F1', 'f2': 'F2', 'f3': 'F3', 'f4': 'F4', 'f5': 'F5', 'f6': 'F6',
  'f7': 'F7', 'f8': 'F8', 'f9': 'F9', 'f10': 'F10', 'f11': 'F11', 'f12': 'F12',
  // Symbols
  '+': 'Equal',
  '=': 'Equal',
  '-': 'Minus',
  '_': 'Minus',
  '[': 'BracketLeft',
  ']': 'BracketRight',
  '{': 'BracketLeft',
  '}': 'BracketRight',
  '\\': 'Backslash',
  '|': 'Backslash',
  ';': 'Semicolon',
  ':': 'Semicolon',
  "'": 'Quote',
  '"': 'Quote',
  ',': 'Comma',
  '<': 'Comma',
  '.': 'Period',
  '>': 'Period',
  '/': 'Slash',
  '?': 'Slash',
  '`': 'Backquote',
  '~': 'Backquote',
};

export function normalizeKeyCode(key: string): string {
  if (!key) return key;
  
  const lower = key.toLowerCase();
  
  // Check direct mapping
  if (KEY_CODE_MAP[lower]) {
    return KEY_CODE_MAP[lower];
  }
  
  // Already in KeyCode format (KeyA, Digit0, etc.)
  if (/^Key[A-Z]$/i.test(key) || /^Digit[0-9]$/i.test(key)) {
    return key.charAt(0).toUpperCase() + key.slice(1);
  }
  
  // Arrow keys
  if (/^Arrow(Up|Down|Left|Right)$/i.test(key)) {
    return 'Arrow' + key.slice(5);
  }
  
  // Function keys
  if (/^F([1-9]|1[0-2])$/i.test(key)) {
    return key.toUpperCase();
  }
  
  // Numpad
  if (/^Numpad/i.test(key)) {
    return 'Numpad' + key.slice(6);
  }
  
  // Single letter -> KeyX
  if (lower.length === 1 && lower >= 'a' && lower <= 'z') {
    return `Key${lower.toUpperCase()}`;
  }
  
  // Single digit -> DigitX
  if (lower.length === 1 && lower >= '0' && lower <= '9') {
    return `Digit${lower}`;
  }
  
  // Return as-is if no match (e.g., already normalized)
  return key;
}

// =============================================================================
// String Parsing & Canonicalization
// =============================================================================

/**
 * Parse a shortcut string like "Ctrl+Shift+S" into modifiers and key
 */
export function parseShortcutString(shortcut: string): { modifiers: ModifierKey[]; key: string } {
  const parts = shortcut.split('+').map(p => p.trim()).filter(Boolean);
  
  if (parts.length === 0) {
    return { modifiers: [], key: '' };
  }
  
  const modifiers: ModifierKey[] = [];
  let key = '';
  
  for (let i = 0; i < parts.length; i++) {
    const part = parts[i];
    const mod = normalizeModifier(part);
    
    if (mod && i < parts.length - 1) {
      // It's a modifier and not the last part
      if (!modifiers.includes(mod)) {
        modifiers.push(mod);
      }
    } else {
      // Last part or not a modifier -> it's the key
      key = normalizeKeyCode(part);
    }
  }
  
  return { modifiers: sortModifiers(modifiers), key };
}

/**
 * Build a canonical shortcut ID from modifiers and key
 */
export function buildCanonicalId(modifiers: ModifierKey[], key: string): string {
  const sortedMods = sortModifiers(modifiers);
  const parts = [...sortedMods, key].filter(Boolean);
  return parts.join('+');
}

/**
 * Canonicalize a shortcut string to standard format
 */
export function canonicalizeShortcut(shortcut: string): string {
  const { modifiers, key } = parseShortcutString(shortcut);
  return buildCanonicalId(modifiers, key);
}

// =============================================================================
// Token Creation
// =============================================================================

export function createKeyboardToken(event: KeyboardEvent): InputToken {
  const modifiers = extractModifiersFromEvent(event);
  const key = event.code || normalizeKeyCode(event.key);
  const id = buildCanonicalId(modifiers, key);
  
  return {
    kind: 'keyboard',
    id,
    raw: id,
    meta: {
      key: event.key,
      code: event.code,
      modifiers,
    },
  };
}

export function createPointerToken(event: PointerEvent | MouseEvent): InputToken {
  const modifiers = extractModifiersFromEvent(event);
  
  let buttonName: string;
  switch (event.button) {
    case 0: buttonName = 'Click'; break;
    case 1: buttonName = 'MiddleClick'; break;
    case 2: buttonName = 'RightClick'; break;
    default: buttonName = `Button${event.button}`;
  }
  
  const pointerType = 'pointerType' in event ? event.pointerType : 'mouse';
  const penPart = pointerType === 'pen' ? 'Pen+' : '';
  const modPart = modifiers.length ? modifiers.join('+') + '+' : '';
  const id = `${modPart}${penPart}${buttonName}`;
  
  return {
    kind: 'pointer',
    id,
    raw: id,
    meta: {
      button: event.button,
      pointerType: pointerType as 'mouse' | 'pen' | 'touch',
      modifiers,
    },
  };
}

export function createWheelToken(event: WheelEvent): InputToken {
  const modifiers = extractModifiersFromEvent(event);
  const direction = event.deltaY < 0 ? 'WheelUp' : 'WheelDown';
  const modPart = modifiers.length ? modifiers.join('+') + '+' : '';
  const id = `${modPart}${direction}`;
  
  return {
    kind: 'wheel',
    id,
    raw: id,
    meta: {
      deltaY: event.deltaY,
      modifiers,
    },
  };
}

export function createGestureToken(gesture: string, meta?: Record<string, unknown>): InputToken {
  return {
    kind: 'gesture',
    id: gesture.toLowerCase(),
    raw: gesture,
    meta,
  };
}

// =============================================================================
// Token Comparison
// =============================================================================

export function tokensEqual(a: InputToken | string, b: InputToken | string): boolean {
  const idA = typeof a === 'string' ? a : a.id;
  const idB = typeof b === 'string' ? b : b.id;
  
  return idA.toLowerCase() === idB.toLowerCase();
}

export function tokenMatchesDefinition(token: InputToken, definitionId: string): boolean {
  return token.id.toLowerCase() === definitionId.toLowerCase();
}

// =============================================================================
// Sequence Normalization
// =============================================================================

/**
 * Normalize a keys definition (string or array) to array of canonical tokens
 */
export function normalizeKeysToTokens(keys: string | string[]): InputToken[] {
  const keyArray = Array.isArray(keys) ? keys : keys.split(/\s+/).filter(Boolean);
  
  return keyArray.map(k => {
    const canonical = canonicalizeShortcut(k);
    const { modifiers, key } = parseShortcutString(k);
    
    // Detect kind based on content
    let kind: TokenKind = 'keyboard';
    
    if (/^(pinch|rotate|swipe)/i.test(k)) {
      kind = 'gesture';
    } else if (/Wheel(Up|Down)$/i.test(k)) {
      kind = 'wheel';
    } else if (/(Click|RightClick|MiddleClick)$/i.test(k)) {
      kind = 'pointer';
    }
    
    return {
      kind,
      id: canonical,
      raw: k,
      meta: { modifiers, key },
    };
  });
}

// =============================================================================
// Display Formatting
// =============================================================================

const MAC_SYMBOLS: Record<string, string> = {
  Meta: '⌘',
  Ctrl: '⌃',
  Alt: '⌥',
  Shift: '⇧',
  Enter: '↩',
  Backspace: '⌫',
  Delete: '⌦',
  Escape: 'esc',
  Tab: '⇥',
  Space: '␣',
  ArrowUp: '↑',
  ArrowDown: '↓',
  ArrowLeft: '←',
  ArrowRight: '→',
};

const WINDOWS_SYMBOLS: Record<string, string> = {
  Meta: 'Win',
  Ctrl: 'Ctrl',
  Alt: 'Alt',
  Shift: 'Shift',
  Enter: 'Enter',
  Backspace: 'Backspace',
  Delete: 'Del',
  Escape: 'Esc',
  Tab: 'Tab',
  Space: 'Space',
  ArrowUp: '↑',
  ArrowDown: '↓',
  ArrowLeft: '←',
  ArrowRight: '→',
};

function formatKeyForDisplay(key: string, platform: Platform): string {
  const symbols = platform === 'mac' ? MAC_SYMBOLS : WINDOWS_SYMBOLS;
  
  if (symbols[key]) {
    return symbols[key];
  }
  
  // Convert KeyX to X, DigitX to X
  if (/^Key([A-Z])$/.test(key)) {
    return key.slice(3);
  }
  if (/^Digit([0-9])$/.test(key)) {
    return key.slice(5);
  }
  
  // Function keys
  if (/^F[0-9]+$/.test(key)) {
    return key;
  }
  
  // Special cases
  if (key === 'Equal') return '+';
  if (key === 'Minus') return '-';
  
  return key;
}

/**
 * Format a shortcut for display in the UI
 */
export function formatShortcutForDisplay(shortcut: string, platform?: Platform): string {
  const plat = platform || detectPlatform();
  const { modifiers, key } = parseShortcutString(shortcut);
  
  const parts = [
    ...modifiers.map(m => formatKeyForDisplay(m, plat)),
    formatKeyForDisplay(key, plat),
  ].filter(Boolean);
  
  if (plat === 'mac') {
    // Mac style: symbols together without separator
    return parts.join('');
  } else {
    // Windows/Linux style: with + separator
    return parts.join('+');
  }
}

/**
 * Get array of formatted parts for Kbd component rendering
 */
export function getShortcutDisplayParts(shortcut: string, platform?: Platform): string[] {
  const plat = platform || detectPlatform();
  const { modifiers, key } = parseShortcutString(shortcut);
  
  return [
    ...modifiers.map(m => formatKeyForDisplay(m, plat)),
    formatKeyForDisplay(key, plat),
  ].filter(Boolean);
}
