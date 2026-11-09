import { describe, expect, it } from 'vitest';
import {
    convertCssColorToHexadecimal,
    resolveComputedCssColor,
    validateHexadecimalColor
} from './color';

describe('color conversion and resolution utilities', () => {
    it('validates 3-digit and 6-digit hexadecimal strings correctly', () => {
        expect(validateHexadecimalColor('#fff')).toBe(true);
        expect(validateHexadecimalColor('#18181b')).toBe(true);
        expect(validateHexadecimalColor('invalid-color')).toBe(false);
    });

    it('expands 3-digit hexadecimals to 6-digit normalized hexadecimals', () => {
        expect(convertCssColorToHexadecimal('#fff')).toBe('#ffffff');
        expect(convertCssColorToHexadecimal('#000')).toBe('#000000');
        expect(convertCssColorToHexadecimal('#abc')).toBe('#aabbcc');
    });

    it('preserves valid 6-digit hexadecimal colors', () => {
        expect(convertCssColorToHexadecimal('#18181b')).toBe('#18181b');
        expect(convertCssColorToHexadecimal('#FF5733')).toBe('#ff5733');
    });

    it('converts modern RGB and OKLCH colors without relying on canvas parsing', () => {
        expect(convertCssColorToHexadecimal('rgb(100% 0% 50%)')).toBe('#ff0080');
        expect(convertCssColorToHexadecimal('oklch(50% 0 0)')).toBe('#636363');
        expect(convertCssColorToHexadecimal('oklch(0.627955 0.257683 29.2345)')).toBe('#ff0000');
    });

    it('resolves CSS custom property variables via probe element', () => {
        document.documentElement.style.setProperty('--test-background-color', 'rgb(24, 24, 27)');

        const resolvedExpression = resolveComputedCssColor('var(--test-background-color)');
        expect(resolvedExpression).toContain('rgb(24, 24, 27)');

        const convertedHexadecimal = convertCssColorToHexadecimal('--test-background-color');
        expect(convertedHexadecimal).toBe('#18181b');

        document.documentElement.style.removeProperty('--test-background-color');
    });
});
