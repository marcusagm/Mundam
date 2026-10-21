import { describe, expect, it } from 'vitest';
import type { RegisteredShortcut, ShortcutPayload } from '../input/types';
import { extractSelectionModifiers } from './gridNavHelpers';

describe('extractSelectionModifiers', () => {
    it('recognizes Ctrl from the normalized shortcut payload', () => {
        const shortcutDefinition: RegisteredShortcut = {
            id: 'test-shortcut',
            name: 'Test Shortcut',
            keys: 'Ctrl+Space',
            defaultKeys: 'Ctrl+Space',
            scope: 'viewport',
            tokens: [],
            normalizedKeys: 'Ctrl+Space'
        };
        const payload: ShortcutPayload = {
            shortcutDef: shortcutDefinition,
            sequence: [],
            meta: { modifiers: ['Ctrl'] }
        };

        expect(extractSelectionModifiers(payload)).toEqual({ multi: true, shift: false });
    });
});
