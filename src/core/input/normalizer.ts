/**
 * Token Normalizer
 * Pure utility functions for normalizing and comparing input tokens.
 * Platform-aware (Mac uses Meta, Windows/Linux use Ctrl for "primary" modifier)
 */

import type { InputToken, ModifierKey, Platform, TokenKind } from './types';
import { detectPlatform, isMac, getPrimaryModifier } from './utils/platform';
import { MODIFIER_ORDER, MODIFIER_ALIASES, KEY_CODE_MAP } from './utils/keys';
import { formatKeyForDisplay } from './utils/display';

export { detectPlatform, isMac, getPrimaryModifier };

// =============================================================================
// Modifier Helpers
// =============================================================================

export function normalizeModifier(mod: string, platform?: Platform): ModifierKey | null {
    const lower = mod.toLowerCase();
    if (lower === 'mod' || lower === 'cmdorctrl' || lower === 'primary') {
        return getPrimaryModifier(platform);
    }
    return MODIFIER_ALIASES[lower] || null;
}

export function sortModifiers(mods: ModifierKey[]): ModifierKey[] {
    return [...mods].sort((a, b) => MODIFIER_ORDER.indexOf(a) - MODIFIER_ORDER.indexOf(b));
}

export function extractModifiersFromEvent(
    event: KeyboardEvent | MouseEvent | WheelEvent
): ModifierKey[] {
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

/**
 * Normalizes a key name to standard Code values (KeyA, Digit0, Escape, etc.)
 */
export function normalizeKeyCode(key: string): string {
    if (!key) return key;

    const lower = key.toLowerCase();

    // 1. Direct mapping from KEY_CODE_MAP
    if (KEY_CODE_MAP[lower]) {
        return KEY_CODE_MAP[lower];
    }

    // 2. Pattern-based normalization
    return applyPatternNormalization(key, lower);
}

/**
 * Helper to reduce complexity of normalizeKeyCode
 */
function normalizeAlphaNumeric(text: string): string | null {
    if (/^Key[A-Z]$/i.test(text) || /^Digit[0-9]$/i.test(text)) {
        return text.charAt(0).toUpperCase() + text.slice(1);
    }
    return null;
}

function normalizeArrowKeys(text: string): string | null {
    if (/^Arrow(Up|Down|Left|Right)$/i.test(text)) {
        const direction = text.slice(5).toLowerCase();
        const capitalized = direction.charAt(0).toUpperCase() + direction.slice(1);
        return `Arrow${capitalized}`;
    }
    return null;
}

function normalizeFunctionKeys(text: string): string | null {
    if (/^F([1-9]|1[0-2])$/i.test(text)) {
        return text.toUpperCase();
    }
    return null;
}

function normalizeNumpadKeys(text: string): string | null {
    if (/^Numpad/i.test(text)) {
        return 'Numpad' + text.slice(6);
    }
    return null;
}

function normalizeSingleCharacter(lower: string): string | null {
    if (lower.length === 1) {
        if (lower >= 'a' && lower <= 'z') {
            return `Key${lower.toUpperCase()}`;
        }
        if (lower >= '0' && lower <= '9') {
            return `Digit${lower}`;
        }
    }
    return null;
}

/**
 * Applies non-alphanumeric patterns for normalization.
 */
function applyNonAlphaPatterns(key: string, lower: string): string {
    const arrowMatch = normalizeArrowKeys(key);
    if (arrowMatch) return arrowMatch;

    const functionMatch = normalizeFunctionKeys(key);
    if (functionMatch) return functionMatch;

    const numpadMatch = normalizeNumpadKeys(key);
    if (numpadMatch) return numpadMatch;

    const singleCharMatch = normalizeSingleCharacter(lower);
    if (singleCharMatch) return singleCharMatch;

    return key;
}

/**
 * Helper to reduce complexity of normalizeKeyCode
 */
function applyPatternNormalization(key: string, lower: string): string {
    const alphaNumericMatch = normalizeAlphaNumeric(key);
    if (alphaNumericMatch) return alphaNumericMatch;

    return applyNonAlphaPatterns(key, lower);
}

// =============================================================================
// String Parsing & Canonicalization
// =============================================================================

/**
 * Parse a shortcut string like "Ctrl+Shift+S" into modifiers and key
 */
export function parseShortcutString(
    shortcut: string,
    platform?: Platform
): { modifiers: ModifierKey[]; key: string } {
    const parts = shortcut
        .split('+')
        .map(p => p.trim())
        .filter(Boolean);

    if (parts.length === 0) {
        return { modifiers: [], key: '' };
    }

    const modifiers: ModifierKey[] = [];
    let key = '';

    for (let index = 0; index < parts.length; index++) {
        const part = parts[index];
        const mod = normalizeModifier(part, platform);

        if (mod && index < parts.length - 1) {
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
export function canonicalizeShortcut(shortcut: string, platform?: Platform): string {
    const { modifiers, key } = parseShortcutString(shortcut, platform);
    return buildCanonicalId(modifiers, key);
}

// =============================================================================
// Token Creation
// =============================================================================

export function resolveKeyboardEventKey(event: KeyboardEvent): string {
    const key = event.code || normalizeKeyCode(event.key);
    return key === 'Backspace' ? 'Delete' : key;
}

export function createKeyboardToken(event: KeyboardEvent): InputToken {
    const modifiers = extractModifiersFromEvent(event);
    const key = resolveKeyboardEventKey(event);
    const id = buildCanonicalId(modifiers, key);

    return {
        kind: 'keyboard',
        id,
        raw: id,
        meta: {
            key: event.key,
            code: event.code,
            modifiers
        }
    };
}

export function createPointerToken(event: PointerEvent | MouseEvent): InputToken {
    const modifiers = extractModifiersFromEvent(event);

    let buttonName: string;
    switch (event.button) {
        case 0:
            buttonName = 'Click';
            break;
        case 1:
            buttonName = 'MiddleClick';
            break;
        case 2:
            buttonName = 'RightClick';
            break;
        case 3:
            buttonName = 'BackClick';
            break;
        case 4:
            buttonName = 'ForwardClick';
            break;
        default:
            buttonName = `Button${event.button}`;
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
            modifiers
        }
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
            modifiers
        }
    };
}

export function createGestureToken(gesture: string, meta?: Record<string, unknown>): InputToken {
    return {
        kind: 'gesture',
        id: gesture.toLowerCase(),
        raw: gesture,
        meta
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
export function normalizeKeysToTokens(keys: string | string[], platform?: Platform): InputToken[] {
    const keyArray = Array.isArray(keys) ? keys : keys.split(/\s+/).filter(Boolean);

    return keyArray.map(keyString => {
        const canonical = canonicalizeShortcut(keyString, platform);
        const { modifiers, key } = parseShortcutString(keyString, platform);

        // Detect kind based on content
        let kind: TokenKind = 'keyboard';
        if (/^(pinch|rotate|swipe)/i.test(keyString)) {
            kind = 'gesture';
        } else if (/Wheel(Up|Down)$/i.test(keyString)) {
            kind = 'wheel';
        } else if (/(Click|RightClick|MiddleClick)$/i.test(keyString)) {
            kind = 'pointer';
        }

        return {
            kind,
            id: canonical,
            raw: keyString,
            meta: { modifiers, key }
        };
    });
}

// =============================================================================
// Display Formatting
// =============================================================================

/**
 * Format a shortcut for display in the UI
 */
export function formatShortcutForDisplay(shortcut: string, platform?: Platform): string {
    const plat = platform || detectPlatform();
    const { modifiers, key } = parseShortcutString(shortcut, plat);

    const parts = [
        ...modifiers.map(m => formatKeyForDisplay(m, plat)),
        formatKeyForDisplay(key, plat)
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
    const { modifiers, key } = parseShortcutString(shortcut, plat);

    return [
        ...modifiers.map(m => formatKeyForDisplay(m, plat)),
        formatKeyForDisplay(key, plat)
    ].filter(Boolean);
}
