import type { Platform } from '../types';
import { MAC_SYMBOLS, WINDOWS_SYMBOLS } from './keys';

const CODE_DISPLAY_MAP: Record<string, string> = {
    Equal: '+',
    Minus: '-',
    Comma: ',',
    Period: '.',
    Slash: '/',
    Semicolon: ';',
    Quote: "'",
    BracketLeft: '[',
    BracketRight: ']',
    Backslash: '\\',
    Backquote: '`'
};

/**
 * Formats a key name for user-facing display based on the platform.
 *
 * @param key - The normalized key code.
 * @param platform - The target platform for formatting.
 * @returns The formatted key string or symbol.
 */
export function formatKeyForDisplay(key: string, platform: Platform): string {
    const symbols = platform === 'mac' ? MAC_SYMBOLS : WINDOWS_SYMBOLS;

    if (symbols[key]) {
        return symbols[key];
    }

    if (CODE_DISPLAY_MAP[key]) {
        return CODE_DISPLAY_MAP[key];
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

    return key;
}
