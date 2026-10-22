import { THEME_TOKENS, sanitizeColors, type CustomTheme } from './customTheme';

/**
 * Finds the custom theme that should be active.
 *
 * @param {string} mode - The configured interface mode.
 * @param {CustomTheme[]} customThemes - Every saved custom theme.
 * @param {string | null} customThemeId - Identifier of the selected custom theme.
 * @returns {CustomTheme | undefined} The active theme, or undefined when mode is not custom.
 */
export function findActiveCustomTheme(
    mode: string,
    customThemes: CustomTheme[],
    customThemeId: string | null
): CustomTheme | undefined {
    if (mode !== 'custom') return undefined;
    return customThemes.find(theme => theme.id === customThemeId);
}

/**
 * Resolves the concrete color scheme (dark or light) for the configured mode.
 *
 * @param {string} mode - The configured interface mode.
 * @param {CustomTheme | undefined} customTheme - The active custom theme, if any.
 * @returns {'dark' | 'light'} The color scheme to set on the document.
 */
export function resolveColorScheme(
    mode: string,
    customTheme: CustomTheme | undefined
): 'dark' | 'light' {
    if (customTheme) return customTheme.base;
    if (mode === 'dark' || mode === 'light') return mode;
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

/**
 * Applies the custom theme colors as inline custom properties on the root element.
 * Inline styles are used so they win over the `:root` and `[data-theme]` rules.
 * Previously applied overrides are always cleared first.
 *
 * @param {HTMLElement} root - The document root element.
 * @param {CustomTheme | undefined} customTheme - The theme to apply, or undefined to clear.
 * @returns {void}
 */
export function applyCustomColors(root: HTMLElement, customTheme: CustomTheme | undefined): void {
    THEME_TOKENS.forEach(token => root.style.removeProperty(token));
    if (!customTheme) return;

    Object.entries(sanitizeColors(customTheme.colors)).forEach(([token, value]) =>
        root.style.setProperty(token, value)
    );
}
