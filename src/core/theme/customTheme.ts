import { z } from 'zod';
import { isSafeCssColor } from '../../utils/color';

/** Identifier written to every exported theme file. */
export const THEME_FILE_SCHEMA = 'mundam-theme';

/** Current version of the theme file format. Files with a greater version are rejected. */
export const THEME_FILE_VERSION = 1;

/** Maximum length of a theme name. */
export const MAXIMUM_THEME_NAME_LENGTH = 60;
const MAXIMUM_THEME_ID_LENGTH = 100;
const THEME_ID_RANDOM_SUFFIX_LENGTH = 4;
const RADIX_BASE_36 = 36;

/**
 * A color token that can be edited by the user.
 */
export interface ThemeTokenDefinition {
    /** CSS custom property name (semantic layer of tokens.css). */
    token: string;
    /** Human readable name shown in the editor. */
    label: string;
}

/**
 * A labeled set of related color tokens shown together in the editor.
 */
export interface ThemeTokenGroup {
    /** Stable group identifier. */
    id: string;
    /** Human readable group name. */
    label: string;
    /** Tokens belonging to the group. */
    tokens: ThemeTokenDefinition[];
}

/**
 * Whitelist of color tokens a custom theme may override.
 * Typography, spacing and geometry tokens are intentionally absent: they are
 * controlled by their own appearance settings and must never come from a theme.
 */
export const THEME_TOKEN_GROUPS: ThemeTokenGroup[] = [
    {
        id: 'surfaces',
        label: 'Backgrounds',
        tokens: [
            { token: '--bg-page', label: 'Page' },
            { token: '--bg-sidebar', label: 'Sidebar' },
            { token: '--bg-header', label: 'Header' },
            { token: '--bg-surface-1', label: 'Surface 1' },
            { token: '--bg-surface-2', label: 'Surface 2' },
            { token: '--bg-surface-3', label: 'Surface 3' },
            { token: '--bg-surface-active', label: 'Active surface' }
        ]
    },
    {
        id: 'borders',
        label: 'Borders',
        tokens: [
            { token: '--border-subtle', label: 'Subtle' },
            { token: '--border-default', label: 'Default' },
            { token: '--border-strong', label: 'Strong' },
            { token: '--border-active', label: 'Active' }
        ]
    },
    {
        id: 'text',
        label: 'Text',
        tokens: [
            { token: '--text-primary', label: 'Primary' },
            { token: '--text-secondary', label: 'Secondary' },
            { token: '--text-tertiary', label: 'Tertiary' },
            { token: '--text-inverse', label: 'Inverse' }
        ]
    },
    {
        id: 'actions',
        label: 'Actions',
        tokens: [
            { token: '--action-primary-bg', label: 'Primary' },
            { token: '--action-primary-bg-hover', label: 'Primary (hover)' },
            { token: '--action-primary-text', label: 'Primary text' },
            { token: '--action-secondary-bg', label: 'Secondary' },
            { token: '--action-secondary-bg-hover', label: 'Secondary (hover)' },
            { token: '--action-secondary-text', label: 'Secondary text' },
            { token: '--action-destructive-bg', label: 'Destructive' },
            { token: '--action-destructive-text', label: 'Destructive text' }
        ]
    },
    {
        id: 'status',
        label: 'Status',
        tokens: [
            { token: '--text-success', label: 'Success text' },
            { token: '--text-warning', label: 'Warning text' },
            { token: '--text-destructive', label: 'Error text' },
            { token: '--text-info', label: 'Info text' },
            { token: '--bg-success', label: 'Success background' },
            { token: '--bg-warning', label: 'Warning background' },
            { token: '--bg-error', label: 'Error background' },
            { token: '--bg-info', label: 'Info background' }
        ]
    }
];

/** Flat list of every customizable token name. */
export const THEME_TOKENS: string[] = THEME_TOKEN_GROUPS.flatMap(group =>
    group.tokens.map(definition => definition.token)
);

const TOKEN_SET = new Set(THEME_TOKENS);

/**
 * Checks whether a CSS custom property may be overridden by a custom theme.
 *
 * @param {string} token - The CSS custom property name.
 * @returns {boolean} True if the token is whitelisted.
 */
export function isThemeToken(token: string): boolean {
    return TOKEN_SET.has(token);
}

/** Built-in mode a custom theme falls back to. */
export type CustomThemeBase = 'dark' | 'light';

/**
 * A user-defined color theme.
 */
export interface CustomTheme {
    /** Unique local identifier. */
    id: string;
    /** Name shown in the theme select. */
    name: string;
    /** Built-in mode whose tokens are used for everything not overridden. */
    base: CustomThemeBase;
    /** Overridden tokens mapped to CSS colors. */
    colors: Record<string, string>;
}

/** Validation schema for a persisted custom theme. */
export const CustomThemeSchema = z.object({
    id: z.string().min(1).max(MAXIMUM_THEME_ID_LENGTH),
    name: z.string().trim().min(1).max(MAXIMUM_THEME_NAME_LENGTH),
    base: z.enum(['dark', 'light']),
    colors: z.record(z.string(), z.string())
});

/** Validation schema for an exported theme file. */
export const ThemeFileSchema = z.object({
    schema: z.literal(THEME_FILE_SCHEMA),
    version: z.number().int().min(1).max(THEME_FILE_VERSION),
    name: z.string().trim().min(1).max(MAXIMUM_THEME_NAME_LENGTH),
    base: z.enum(['dark', 'light']),
    colors: z.record(z.string(), z.string())
});

/**
 * Drops unknown tokens and unsafe values.
 *
 * @param {Record<string, string>} colors - Raw token to color map.
 * @returns {Record<string, string>} Only whitelisted tokens with safe color values.
 */
export function sanitizeColors(colors: Record<string, string>): Record<string, string> {
    const result: Record<string, string> = {};
    for (const [token, value] of Object.entries(colors)) {
        if (isThemeToken(token) && typeof value === 'string' && isSafeCssColor(value)) {
            result[token] = value.trim();
        }
    }
    return result;
}

/**
 * Generates a unique identifier for a custom theme.
 *
 * @returns {string} The generated identifier.
 */
export function createThemeId(): string {
    return `custom-${Date.now().toString(RADIX_BASE_36)}-${Math.random()
        .toString(RADIX_BASE_36)
        .slice(2, 2 + THEME_ID_RANDOM_SUFFIX_LENGTH)}`;
}

/**
 * Serializes a theme to the shareable JSON file format.
 *
 * @param {CustomTheme} theme - The theme to export.
 * @returns {string} Pretty-printed JSON content.
 */
export function serializeTheme(theme: CustomTheme): string {
    return JSON.stringify(
        {
            schema: THEME_FILE_SCHEMA,
            version: THEME_FILE_VERSION,
            name: theme.name,
            base: theme.base,
            colors: sanitizeColors(theme.colors)
        },
        null,
        2
    );
}

/** Outcome of parsing an imported theme file. */
export type ParseThemeResult = { ok: true; theme: CustomTheme } | { ok: false; error: string };

/**
 * Parses an imported theme file. A fresh id is always generated.
 *
 * @param {string} content - Raw file content.
 * @returns {ParseThemeResult} The theme, or an error message.
 */
export function parseThemeFile(content: string): ParseThemeResult {
    let raw: unknown;
    try {
        raw = JSON.parse(content);
    } catch {
        return { ok: false, error: 'The file is not valid JSON.' };
    }
    const parsed = ThemeFileSchema.safeParse(raw);
    if (!parsed.success) {
        return { ok: false, error: 'The file is not a valid Mundam theme.' };
    }
    return {
        ok: true,
        theme: {
            id: createThemeId(),
            name: parsed.data.name,
            base: parsed.data.base,
            colors: sanitizeColors(parsed.data.colors)
        }
    };
}

/**
 * Parses the persisted theme list, ignoring invalid entries.
 *
 * @param {string | null} content - Raw JSON stored in settings.
 * @returns {CustomTheme[]} The valid themes.
 */
export function parseStoredThemes(content: string | null): CustomTheme[] {
    if (!content) return [];
    try {
        const raw = JSON.parse(content);
        if (!Array.isArray(raw)) return [];
        const themes: CustomTheme[] = [];
        for (const entry of raw) {
            const parsed = CustomThemeSchema.safeParse(entry);
            if (parsed.success) {
                themes.push({ ...parsed.data, colors: sanitizeColors(parsed.data.colors) });
            }
        }
        return themes;
    } catch {
        return [];
    }
}
