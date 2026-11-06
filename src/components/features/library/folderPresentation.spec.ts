import { describe, expect, it } from 'vitest';
import { resolveFolderColor } from './folderPresentation';
import { isFolderDescendant } from './folderHierarchy';

describe('folder presentation', () => {
    const folders = [
        { id: 'root', parent_id: null, color: '#112233', inherits_color: false },
        { id: 'child', parent_id: 'root', color: '#445566', inherits_color: true },
        { id: 'grandchild', parent_id: 'child', inherits_color: true },
        { id: 'default', parent_id: 'root', color: null, inherits_color: false }
    ];
    it('inherits the live ancestor choice and supports an explicit default', () => {
        expect(resolveFolderColor('grandchild', folders)).toBe('#112233');
        expect(resolveFolderColor('default', folders)).toBeUndefined();
        expect(
            resolveFolderColor('child', [{ ...folders[0], color: '#778899' }, ...folders.slice(1)])
        ).toBe('#778899');
    });
    it('excludes the folder itself and every descendant as move destinations', () => {
        expect(isFolderDescendant('grandchild', 'child', folders)).toBe(true);
        expect(isFolderDescendant('child', 'child', folders)).toBe(true);
        expect(isFolderDescendant('root', 'child', folders)).toBe(false);
    });
    it('terminates when an invalid hierarchy contains a cycle', () => {
        const cyclic = [
            { id: 'a', parent_id: 'b', inherits_color: true },
            { id: 'b', parent_id: 'a', inherits_color: true }
        ];
        expect(resolveFolderColor('a', cyclic)).toBeUndefined();
        expect(isFolderDescendant('a', 'unknown', cyclic)).toBe(false);
    });
});
