import { Component, Show, createMemo } from 'solid-js';
import { open, save } from '@tauri-apps/plugin-dialog';
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs';
import { Button } from '../../ui/Button';
import { Select } from '../../ui/Select';
import { toast } from '../../ui/Sonner';
import { appearance, appearanceActions } from '../../../core/store/appearanceStore';
import { serializeTheme, type CustomThemeBase } from '../../../core/theme/customTheme';
import { formatSlug } from '../../../utils/format';
import { CustomThemeColorFields } from './CustomThemeColorFields';

const THEME_FILE_FILTERS = [{ name: 'Mundam Theme', extensions: ['json'] }];

/**
 * Toolbar and editor for custom color themes: select, create, duplicate,
 * import, export, delete and edit colors.
 *
 * @returns {JSX.Element} The rendered editor.
 *
 * @example
 * ```tsx
 * <Show when={appearance().mode === 'custom'}>
 *     <CustomThemeEditor />
 * </Show>
 * ```
 */
export const CustomThemeEditor: Component = () => {
    const activeTheme = () =>
        appearance().customThemes.find(theme => theme.id === appearance().customThemeId);

    const themeOptions = createMemo(() =>
        appearance().customThemes.map(theme => ({ value: theme.id, label: theme.name }))
    );

    /**
     * Creates a theme from the active one (duplicate) or from the current built-in mode.
     *
     * @returns {Promise<void>}
     */
    const handleCreate = async (): Promise<void> => {
        const sourceTheme = activeTheme();
        const currentScheme: CustomThemeBase =
            document.documentElement.getAttribute('data-theme') === 'light' ? 'light' : 'dark';
        const name = sourceTheme
            ? `${sourceTheme.name} copy`
            : `Custom theme ${appearance().customThemes.length + 1}`;
        await appearanceActions.createTheme(name, sourceTheme?.base ?? currentScheme, {
            ...sourceTheme?.colors
        });
    };

    /**
     * Lets the user pick a theme file and imports it.
     *
     * @returns {Promise<void>}
     */
    const handleImport = async (): Promise<void> => {
        try {
            const path = await open({ multiple: false, filters: THEME_FILE_FILTERS });
            if (typeof path !== 'string') return;
            const result = await appearanceActions.importTheme(await readTextFile(path));
            if (result.ok) toast.success(`Theme "${result.theme.name}" imported`);
            else toast.error('Could not import theme', { description: result.error });
        } catch (error) {
            toast.error('Could not import theme', { description: String(error) });
        }
    };

    /**
     * Lets the user choose a destination and exports the active theme.
     *
     * @returns {Promise<void>}
     */
    const handleExport = async (): Promise<void> => {
        const theme = activeTheme();
        if (!theme) return;
        try {
            const path = await save({
                defaultPath: `${formatSlug(theme.name, 'theme')}.mundam-theme.json`,
                filters: THEME_FILE_FILTERS
            });
            if (!path) return;
            await writeTextFile(path, serializeTheme(theme));
            toast.success(`Theme "${theme.name}" exported`);
        } catch (error) {
            toast.error('Could not export theme', { description: String(error) });
        }
    };

    /**
     * Deletes the active theme.
     *
     * @returns {void}
     */
    const handleDelete = (): void => {
        const theme = activeTheme();
        if (theme) void appearanceActions.deleteTheme(theme.id);
    };

    return (
        <>
            <div class="appearance-setting-row">
                <span class="setting-label">Custom Theme:</span>
                <div class="appearance-theme-actions">
                    <div class="appearance-theme-select">
                        <Select
                            options={themeOptions()}
                            value={appearance().customThemeId ?? undefined}
                            placeholder="No themes yet"
                            onValueChange={themeId =>
                                appearanceActions.update({ mode: 'custom', customThemeId: themeId })
                            }
                            size="sm"
                        />
                    </div>
                    <Button size="sm" variant="secondary" onClick={handleCreate}>
                        {activeTheme() ? 'Duplicate' : 'New'}
                    </Button>
                    <Button size="sm" variant="secondary" onClick={handleImport}>
                        Import
                    </Button>
                    <Button
                        size="sm"
                        variant="secondary"
                        disabled={!activeTheme()}
                        onClick={handleExport}
                    >
                        Export
                    </Button>
                    <Button
                        size="sm"
                        variant="ghost-destructive"
                        disabled={!activeTheme()}
                        onClick={handleDelete}
                    >
                        Delete
                    </Button>
                </div>
            </div>

            <Show when={activeTheme()?.id} keyed>
                {themeId => <CustomThemeColorFields themeId={themeId} />}
            </Show>
        </>
    );
};
