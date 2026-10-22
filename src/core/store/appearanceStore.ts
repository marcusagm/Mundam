import { createSignal } from 'solid-js';
import { tauriService } from '../tauri/services';
import { emit, listen } from '@tauri-apps/api/event';
import { ActionResult, ErrorCode } from '../types/actions';
import { AppearancePayloadSchema, type AppearancePayload } from './settings/schemas';
import {
    createThemeId,
    parseStoredThemes,
    parseThemeFile,
    sanitizeColors,
    type CustomTheme,
    type CustomThemeBase,
    type ParseThemeResult
} from '../theme/customTheme';
import {
    applyCustomColors,
    findActiveCustomTheme,
    resolveColorScheme
} from '../theme/applyCustomTheme';

export type ThemeMode = 'dark' | 'light' | 'system' | 'custom';
export type ThemeColor =
    | 'neutral'
    | 'blue'
    | 'emerald'
    | 'orange'
    | 'rose'
    | 'violet'
    | 'teal'
    | 'zinc'
    | 'indigo'
    | 'fuchsia'
    | 'slate'
    | 'stone';
export type ThemeFontSize = 'small' | 'medium' | 'large';

export interface AppearanceState {
    mode: ThemeMode;
    /** Id of the active custom theme (used when mode is 'custom'). */
    customThemeId: string | null;
    customThemes: CustomTheme[];
    theme: ThemeColor;
    radius: number; // in px
    fontSize: ThemeFontSize;
}

const DEFAULT_STATE: AppearanceState = {
    mode: 'system',
    customThemeId: null,
    customThemes: [],
    theme: 'neutral',
    radius: 6,
    fontSize: 'medium'
};

const [appearance, setAppearance] = createSignal<AppearanceState>(DEFAULT_STATE);

/** Incremented after every apply(), so computed-style readers can re-evaluate. */
const [appliedVersion, setAppliedVersion] = createSignal(0);

// Event name for cross-window sync
const SYNC_EVENT = 'appearance:sync';

const CUSTOM_THEMES_KEY = 'appearance_custom_themes';
const CUSTOM_THEME_ID_KEY = 'appearance_custom_theme_id';

const PERSIST_THEMES_DEBOUNCE_MILLISECONDS = 300;

let persistThemesTimer: ReturnType<typeof setTimeout> | null = null;

/** Stored unlisten function for the cross-window sync listener */
let unlistenSyncEvent: (() => void) | null = null;

export const appearanceActions = {
    initialize: async () => {
        // Load individual settings to match "GeneralPanel" pattern
        const [mode, theme, radius, fontSize, customThemes, customThemeId] = await Promise.all([
            tauriService.getSetting('appearance_mode'),
            tauriService.getSetting('appearance_theme'),
            tauriService.getSetting('appearance_radius'),
            tauriService.getSetting('appearance_font_size'),
            tauriService.getSetting(CUSTOM_THEMES_KEY),
            tauriService.getSetting(CUSTOM_THEME_ID_KEY)
        ]);

        setAppearance({
            mode: (mode as ThemeMode) ?? DEFAULT_STATE.mode,
            customThemes: parseStoredThemes(customThemes),
            customThemeId: customThemeId || null,
            theme: (theme as ThemeColor) ?? DEFAULT_STATE.theme,
            radius: radius !== null ? Number(radius) : DEFAULT_STATE.radius,
            fontSize: (fontSize as ThemeFontSize) ?? DEFAULT_STATE.fontSize
        });

        appearanceActions.apply();

        // Listen for changes from other windows (with cleanup)
        // Dispose the previous listener before re-registering
        if (unlistenSyncEvent) {
            unlistenSyncEvent();
            unlistenSyncEvent = null;
        }
        listen<AppearanceState>(SYNC_EVENT, event => {
            setAppearance(event.payload);
            appearanceActions.apply();
        }).then(unlisten => {
            unlistenSyncEvent = unlisten;
        });
    },

    update: async (updates: AppearancePayload): Promise<ActionResult> => {
        const validation = AppearancePayloadSchema.safeParse(updates);
        if (!validation.success) {
            return {
                success: false,
                error: {
                    code: ErrorCode.VALIDATION_ERROR,
                    message: 'Invalid appearance settings'
                }
            };
        }

        const next = { ...appearance(), ...updates };
        setAppearance(next);
        appearanceActions.apply();

        // Persist individually
        const promises: Promise<void>[] = [];
        if (updates.mode) promises.push(tauriService.setSetting('appearance_mode', updates.mode));
        if (updates.customThemeId !== undefined)
            promises.push(
                tauriService.setSetting(CUSTOM_THEME_ID_KEY, updates.customThemeId ?? '')
            );
        if (updates.theme)
            promises.push(tauriService.setSetting('appearance_theme', updates.theme));
        if (updates.radius !== undefined)
            promises.push(tauriService.setSetting('appearance_radius', String(updates.radius)));
        if (updates.fontSize)
            promises.push(tauriService.setSetting('appearance_font_size', updates.fontSize));

        try {
            await Promise.all(promises);
            await emit(SYNC_EVENT, next);
            return { success: true, data: undefined };
        } catch (error) {
            console.error('Failed to save appearance settings:', error);
            return {
                success: false,
                error: {
                    code: ErrorCode.IO_ERROR,
                    message: 'Failed to save appearance settings'
                }
            };
        }
    },

    /**
     * Creates or replaces a custom theme. Applies immediately and persists debounced,
     * so dragging a color picker does not write to disk on every frame.
     *
     * @param {CustomTheme} theme - The theme to store.
     * @returns {void}
     */
    saveTheme: (theme: CustomTheme): void => {
        const sanitized = { ...theme, colors: sanitizeColors(theme.colors) };
        const current = appearance().customThemes;
        const exists = current.some(item => item.id === sanitized.id);
        const customThemes = exists
            ? current.map(item => (item.id === sanitized.id ? sanitized : item))
            : [...current, sanitized];
        setAppearance({ ...appearance(), customThemes });
        appearanceActions.apply();

        if (persistThemesTimer) clearTimeout(persistThemesTimer);
        persistThemesTimer = setTimeout(() => {
            persistThemesTimer = null;
            void appearanceActions.persistThemes();
        }, PERSIST_THEMES_DEBOUNCE_MILLISECONDS);
    },

    /**
     * Persists the custom theme list and notifies other windows.
     *
     * @returns {Promise<ActionResult>} The persistence outcome.
     */
    persistThemes: async (): Promise<ActionResult> => {
        const state = appearance();
        try {
            await tauriService.setSetting(CUSTOM_THEMES_KEY, JSON.stringify(state.customThemes));
            await emit(SYNC_EVENT, state);
            return { success: true, data: undefined };
        } catch (error) {
            console.error('Failed to save custom themes:', error);
            return {
                success: false,
                error: { code: ErrorCode.IO_ERROR, message: 'Failed to save custom themes' }
            };
        }
    },

    /**
     * Creates a new theme, selects it and persists it.
     *
     * @param {string} name - Theme name.
     * @param {CustomThemeBase} base - Built-in mode used as fallback.
     * @param {Record<string, string>} [colors] - Initial color overrides.
     * @returns {Promise<CustomTheme>} The created theme.
     */
    createTheme: async (
        name: string,
        base: CustomThemeBase,
        colors: Record<string, string> = {}
    ): Promise<CustomTheme> => {
        const theme: CustomTheme = { id: createThemeId(), name, base, colors };
        appearanceActions.saveTheme(theme);
        await appearanceActions.update({ mode: 'custom', customThemeId: theme.id });
        await appearanceActions.persistThemes();
        return theme;
    },

    /**
     * Deletes a theme. When it was active, the mode falls back to `system`.
     *
     * @param {string} themeId - Identifier of the theme to delete.
     * @returns {Promise<void>}
     */
    deleteTheme: async (themeId: string): Promise<void> => {
        const state = appearance();
        const customThemes = state.customThemes.filter(item => item.id !== themeId);
        const wasActive = state.customThemeId === themeId;
        setAppearance({
            ...state,
            customThemes,
            customThemeId: wasActive ? null : state.customThemeId
        });
        await appearanceActions.persistThemes();
        if (wasActive) {
            await appearanceActions.update({ mode: 'system', customThemeId: null });
        } else {
            appearanceActions.apply();
        }
    },

    /**
     * Imports a theme file, selects the new theme and persists it.
     *
     * @param {string} content - Raw theme file content.
     * @returns {Promise<ParseThemeResult>} The imported theme, or a validation error.
     */
    importTheme: async (content: string): Promise<ParseThemeResult> => {
        const result = parseThemeFile(content);
        if (!result.ok) return result;
        appearanceActions.saveTheme(result.theme);
        await appearanceActions.update({ mode: 'custom', customThemeId: result.theme.id });
        await appearanceActions.persistThemes();
        return result;
    },

    /**
     * Applies the current appearance state to the document root.
     *
     * @returns {void}
     */
    apply: (): void => {
        const state = appearance();
        const root = document.documentElement;

        const customTheme = findActiveCustomTheme(
            state.mode,
            state.customThemes,
            state.customThemeId
        );
        root.setAttribute('data-theme', resolveColorScheme(state.mode, customTheme));
        applyCustomColors(root, customTheme);

        const themes: ThemeColor[] = [
            'neutral',
            'blue',
            'emerald',
            'orange',
            'rose',
            'violet',
            'teal',
            'zinc',
            'indigo',
            'fuchsia',
            'slate',
            'stone'
        ];
        themes.forEach(themeColor => root.classList.remove(`theme-${themeColor}`));
        root.classList.add(`theme-${state.theme}`);

        root.style.setProperty('--radius-m', `${state.radius}px`);
        root.style.setProperty('--radius-s', `${Math.max(0, state.radius - 2)}px`);
        root.style.setProperty('--radius-xs', `${Math.max(0, state.radius - 4)}px`);
        root.style.setProperty('--radius-l', `${state.radius + 2}px`);

        root.classList.remove('font-size-small', 'font-size-medium', 'font-size-large');
        root.classList.add(`font-size-${state.fontSize}`);

        setAppliedVersion(version => version + 1);
    }
};

window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
    const { mode, customThemeId } = appearance();
    if (mode === 'system' || (mode === 'custom' && !customThemeId)) {
        appearanceActions.apply();
    }
});

export { appearance, appliedVersion };
