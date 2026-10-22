import { Component, For, Show } from 'solid-js';
import { Button } from '../../ui/Button';
import { ColorInput } from '../../ui/ColorInput';
import { Input } from '../../ui/Input';
import { RadioGroup, RadioGroupItem } from '../../ui/RadioGroup';
import { appearance, appearanceActions, appliedVersion } from '../../../core/store/appearanceStore';
import {
    MAXIMUM_THEME_NAME_LENGTH,
    THEME_TOKEN_GROUPS,
    type CustomThemeBase
} from '../../../core/theme/customTheme';
import { convertCssColorToHexadecimal } from '../../../utils/color';

const FALLBACK_HEXADECIMAL_COLOR = '#000000';

/**
 * Properties for the CustomThemeColorFields component.
 */
export interface CustomThemeColorFieldsProperties {
    /** Identifier of the custom theme being edited. */
    themeId: string;
}

/**
 * Editor for the name, base mode and color tokens of a single custom theme.
 *
 * @param {CustomThemeColorFieldsProperties} componentProperties - Component properties.
 * @returns {JSX.Element} The rendered fields.
 *
 * @example
 * ```tsx
 * <CustomThemeColorFields themeId={appearance().customThemeId} />
 * ```
 */
export const CustomThemeColorFields: Component<
    CustomThemeColorFieldsProperties
> = componentProperties => {
    const theme = () =>
        appearance().customThemes.find(item => item.id === componentProperties.themeId);

    /**
     * Resolves the color currently shown for a token, including inherited values.
     *
     * @param {string} token - The CSS custom property name.
     * @returns {string} A hexadecimal color.
     */
    const getEffectiveHexadecimalColor = (token: string): string => {
        appliedVersion();
        const cssColor =
            theme()?.colors[token] ??
            getComputedStyle(document.documentElement).getPropertyValue(token).trim();
        return (cssColor && convertCssColorToHexadecimal(cssColor)) || FALLBACK_HEXADECIMAL_COLOR;
    };

    /**
     * Stores a new color for a token.
     *
     * @param {string} token - The CSS custom property name.
     * @param {string} color - The hexadecimal color.
     * @returns {void}
     */
    const handleColorChange = (token: string, color: string): void => {
        const currentTheme = theme();
        if (!currentTheme || currentTheme.colors[token] === color) return;
        appearanceActions.saveTheme({
            ...currentTheme,
            colors: { ...currentTheme.colors, [token]: color }
        });
    };

    /**
     * Removes the override of a token so it inherits from the base mode again.
     *
     * @param {string} token - The CSS custom property name.
     * @returns {void}
     */
    const handleColorReset = (token: string): void => {
        const currentTheme = theme();
        if (!currentTheme) return;
        const colors = { ...currentTheme.colors };
        delete colors[token];
        appearanceActions.saveTheme({ ...currentTheme, colors });
    };

    /**
     * Renames the theme, ignoring empty names.
     *
     * @param {string} rawName - The typed name.
     * @returns {void}
     */
    const handleNameChange = (rawName: string): void => {
        const currentTheme = theme();
        const name = rawName.trim();
        if (currentTheme && name) appearanceActions.saveTheme({ ...currentTheme, name });
    };

    /**
     * Changes the built-in mode used as fallback.
     *
     * @param {string} base - The selected base mode.
     * @returns {void}
     */
    const handleBaseChange = (base: string): void => {
        const currentTheme = theme();
        if (currentTheme)
            appearanceActions.saveTheme({ ...currentTheme, base: base as CustomThemeBase });
    };

    return (
        <>
            <div class="appearance-setting-row">
                <span class="setting-label">Name:</span>
                <div class="appearance-theme-name">
                    <Input
                        size="sm"
                        value={theme()?.name}
                        maxLength={MAXIMUM_THEME_NAME_LENGTH}
                        onChange={event => handleNameChange(event.currentTarget.value)}
                    />
                </div>
            </div>
            <div class="appearance-setting-row">
                <span class="setting-label">Base (unset colors fall back to):</span>
                <RadioGroup
                    value={theme()?.base}
                    onValueChange={handleBaseChange}
                    orientation="horizontal"
                >
                    <RadioGroupItem value="dark" label="Dark" />
                    <RadioGroupItem value="light" label="Light" />
                </RadioGroup>
            </div>
            <For each={THEME_TOKEN_GROUPS}>
                {group => (
                    <div class="appearance-setting-row vertical">
                        <span class="setting-label">{group.label}</span>
                        <div class="appearance-token-grid">
                            <For each={group.tokens}>
                                {definition => (
                                    <div class="appearance-token">
                                        <ColorInput
                                            size="sm"
                                            label={definition.label}
                                            value={getEffectiveHexadecimalColor(definition.token)}
                                            onChange={color =>
                                                handleColorChange(definition.token, color)
                                            }
                                        />
                                        <Show when={theme()?.colors[definition.token]}>
                                            <Button
                                                size="xs"
                                                variant="ghost"
                                                onClick={() => handleColorReset(definition.token)}
                                            >
                                                Reset
                                            </Button>
                                        </Show>
                                    </div>
                                )}
                            </For>
                        </div>
                    </div>
                )}
            </For>
        </>
    );
};
