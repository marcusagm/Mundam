import { describe, expect, it, afterEach } from 'vitest';
import {
    createKeyboardToken,
    createPointerToken,
    normalizeKeysToTokens,
    canonicalizeShortcut,
    parseShortcutString,
    formatShortcutForDisplay,
    tokensEqual
} from './normalizer';
import { setPlatformForTesting } from './utils/platform';

describe('delete key normalization', () => {
    it('treats Backspace (the macOS Delete key) as the Delete shortcut', () => {
        const token = createKeyboardToken(new KeyboardEvent('keydown', { code: 'Backspace' }));

        expect(token.id).toBe('Delete');
        expect(tokensEqual(normalizeKeysToTokens('Delete')[0], token)).toBe(true);
        expect(normalizeKeysToTokens('Backspace')[0].id).toBe('Delete');
    });
});

describe('pointer shortcut normalization', () => {
    it('maps the mouse back button to the back history token', () => {
        const token = createPointerToken(new MouseEvent('pointerdown', { button: 3 }));

        expect(token.id).toBe('BackClick');
        expect(tokensEqual(normalizeKeysToTokens('BackClick')[0], token)).toBe(true);
    });

    it('maps the mouse forward button to the forward history token', () => {
        const token = createPointerToken(new MouseEvent('pointerdown', { button: 4 }));

        expect(token.id).toBe('ForwardClick');
        expect(tokensEqual(normalizeKeysToTokens('ForwardClick')[0], token)).toBe(true);
    });
});

describe('platform-aware Mod modifier normalization', () => {
    afterEach(() => {
        setPlatformForTesting(null);
    });

    it('resolves Mod to Meta on macOS', () => {
        setPlatformForTesting('mac');

        expect(canonicalizeShortcut('Mod+KeyK')).toBe('Meta+KeyK');
        expect(canonicalizeShortcut('Mod+KeyB')).toBe('Meta+KeyB');
        expect(canonicalizeShortcut('Mod+Comma')).toBe('Meta+Comma');

        const parsed = parseShortcutString('Mod+KeyK');
        expect(parsed.modifiers).toEqual(['Meta']);
        expect(parsed.key).toBe('KeyK');

        const tokens = normalizeKeysToTokens('Mod+KeyK');
        expect(tokens[0].id).toBe('Meta+KeyK');
    });

    it('resolves Mod to Ctrl on Windows and Linux', () => {
        setPlatformForTesting('windows');

        expect(canonicalizeShortcut('Mod+KeyK')).toBe('Ctrl+KeyK');
        expect(canonicalizeShortcut('Mod+KeyB')).toBe('Ctrl+KeyB');
        expect(canonicalizeShortcut('Mod+Comma')).toBe('Ctrl+Comma');

        const parsed = parseShortcutString('Mod+KeyK');
        expect(parsed.modifiers).toEqual(['Ctrl']);
        expect(parsed.key).toBe('KeyK');

        const tokens = normalizeKeysToTokens('Mod+KeyK');
        expect(tokens[0].id).toBe('Ctrl+KeyK');

        setPlatformForTesting('linux');
        expect(canonicalizeShortcut('Mod+KeyK')).toBe('Ctrl+KeyK');
    });

    it('formats Mod shortcuts for display per platform', () => {
        expect(formatShortcutForDisplay('Mod+KeyK', 'mac')).toBe('⌘K');
        expect(formatShortcutForDisplay('Mod+KeyB', 'mac')).toBe('⌘B');
        expect(formatShortcutForDisplay('Mod+Comma', 'mac')).toBe('⌘,');

        expect(formatShortcutForDisplay('Mod+KeyK', 'windows')).toBe('Ctrl+K');
        expect(formatShortcutForDisplay('Mod+KeyB', 'windows')).toBe('Ctrl+B');
        expect(formatShortcutForDisplay('Mod+Comma', 'windows')).toBe('Ctrl+,');
        expect(formatShortcutForDisplay('Mod+Digit1', 'windows')).toBe('Ctrl+1');
    });

    it('matches physical keyboard events on Windows and Mac', () => {
        setPlatformForTesting('windows');
        const windowsEvent = new KeyboardEvent('keydown', {
            code: 'KeyK',
            ctrlKey: true,
            metaKey: false
        });
        const windowsToken = createKeyboardToken(windowsEvent);
        expect(windowsToken.id).toBe('Ctrl+KeyK');
        expect(tokensEqual(windowsToken, normalizeKeysToTokens('Mod+KeyK')[0])).toBe(true);

        setPlatformForTesting('mac');
        const macEvent = new KeyboardEvent('keydown', {
            code: 'KeyK',
            metaKey: true,
            ctrlKey: false
        });
        const macToken = createKeyboardToken(macEvent);
        expect(macToken.id).toBe('Meta+KeyK');
        expect(tokensEqual(macToken, normalizeKeysToTokens('Mod+KeyK')[0])).toBe(true);
    });
});
