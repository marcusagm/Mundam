import { describe, expect, it } from 'vitest';
import {
    createKeyboardToken,
    createPointerToken,
    normalizeKeysToTokens,
    tokensEqual
} from './normalizer';

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
