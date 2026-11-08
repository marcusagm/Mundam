import { describe, expect, it } from 'vitest';
import { buildOrderedScopeGroups, groupShortcutsByScope } from './shortcutScopeGroups';
import type { RegisteredShortcut } from '../../../../core/input/types';

describe('shortcut scope grouping and deduplication', () => {
    it('groups shortcuts by scope and removes duplicate commands', () => {
        const sampleShortcuts: RegisteredShortcut[] = [
            {
                id: 'sc_1',
                name: 'Go to Asset Gallery',
                description: 'Switch to the Asset Gallery view',
                keys: 'Mod+Digit2',
                defaultKeys: 'Mod+Digit2',
                tokens: [],
                normalizedKeys: 'Mod+Digit2',
                scope: 'global',
                command: 'navigate:gallery',
                priority: 0,
                ignoreInputs: false,
                preventDefault: true,
                isDefault: true
            },
            {
                id: 'sc_2',
                name: 'Go to Asset Gallery',
                description: 'Switch to the Asset Gallery view',
                keys: 'Mod+Digit2',
                defaultKeys: 'Mod+Digit2',
                tokens: [],
                normalizedKeys: 'Mod+Digit2',
                scope: 'global',
                command: 'navigate:gallery',
                priority: 0,
                ignoreInputs: false,
                preventDefault: true,
                isDefault: true
            },
            {
                id: 'sc_3',
                name: 'Zoom In',
                description: 'Zoom in image',
                keys: 'Equal',
                defaultKeys: 'Equal',
                tokens: [],
                normalizedKeys: 'Equal',
                scope: 'image-viewer',
                command: 'image:zoom-in',
                priority: 0,
                ignoreInputs: false,
                preventDefault: true,
                isDefault: true
            }
        ];

        const groupedResult = groupShortcutsByScope(sampleShortcuts);
        const globalGroup = groupedResult.get('global');
        expect(globalGroup).toBeDefined();
        expect(globalGroup).toHaveLength(1);
        expect(globalGroup![0].id).toBe('sc_1');

        const orderedGroups = buildOrderedScopeGroups(groupedResult);
        expect(orderedGroups).toHaveLength(2);
        expect(orderedGroups[0].scope).toBe('global');
        expect(orderedGroups[0].shortcuts).toHaveLength(1);
    });
});
