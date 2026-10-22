import { isSafeCssColor } from '../../utils/color';
import { describe, expect, it } from 'vitest';
import { parseStoredThemes, parseThemeFile, sanitizeColors, serializeTheme } from './customTheme';

describe('customTheme', () => {
    it('accepts safe colors and rejects injections', () => {
        expect(isSafeCssColor('#1a2b3c')).toBe(true);
        expect(isSafeCssColor('oklch(50% 0.1 200 / 0.5)')).toBe(true);
        expect(isSafeCssColor('red; background: url(x)')).toBe(false);
        expect(isSafeCssColor('url(http://x)')).toBe(false);
        expect(isSafeCssColor('var(--x)')).toBe(false);
    });

    it('drops tokens outside the whitelist', () => {
        const colors = sanitizeColors({
            '--bg-page': '#000000',
            '--p-font-size-m': '#fff',
            '--radius-m': '#fff',
            '--text-primary': 'javascript:alert(1)'
        });
        expect(colors).toEqual({ '--bg-page': '#000000' });
    });

    it('round-trips through export and import with a fresh id', () => {
        const theme = {
            id: 'a',
            name: 'Ocean',
            base: 'dark' as const,
            colors: { '--bg-page': '#001122' }
        };
        const result = parseThemeFile(serializeTheme(theme));
        expect(result.ok).toBe(true);
        if (result.ok) {
            expect(result.theme.id).not.toBe('a');
            expect(result.theme.name).toBe('Ocean');
            expect(result.theme.colors).toEqual(theme.colors);
        }
    });

    it('rejects invalid files', () => {
        expect(parseThemeFile('nope').ok).toBe(false);
        expect(parseThemeFile('{"schema":"other"}').ok).toBe(false);
    });

    it('ignores invalid stored entries', () => {
        expect(parseStoredThemes('[{"id":"x"}]')).toEqual([]);
        expect(parseStoredThemes(null)).toEqual([]);
    });
});
