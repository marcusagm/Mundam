import { Component, For, Show } from 'solid-js';
import { Sun, Moon, Monitor, Palette } from 'lucide-solid';
import { CustomThemeEditor } from './CustomThemeEditor';
import { SectionGroup } from '../../ui/SectionGroup';
import { Slider } from '../../ui/Slider';
import { ToggleGroup, ToggleGroupItem } from '../../ui/Toggle';
import {
    appearance,
    appearanceActions,
    type ThemeColor,
    type ThemeFontSize,
    type ThemeMode
} from '../../../core/store/appearanceStore';
import './appearance-panel.css';

interface AccentThemeOption {
    identifier: ThemeColor;
    label: string;
    swatchColor: string;
}

const ACCENT_THEME_OPTIONS: AccentThemeOption[] = [
    { identifier: 'neutral', label: 'Neutral', swatchColor: '#737373' },
    { identifier: 'zinc', label: 'Zinc', swatchColor: '#71717a' },
    { identifier: 'slate', label: 'Slate', swatchColor: '#64748b' },
    { identifier: 'stone', label: 'Stone', swatchColor: '#78716c' },
    { identifier: 'blue', label: 'Blue', swatchColor: '#3b82f6' },
    { identifier: 'indigo', label: 'Indigo', swatchColor: '#6366f1' },
    { identifier: 'violet', label: 'Violet', swatchColor: '#8b5cf6' },
    { identifier: 'fuchsia', label: 'Fuchsia', swatchColor: '#d946ef' },
    { identifier: 'rose', label: 'Rose', swatchColor: '#f43f5e' },
    { identifier: 'orange', label: 'Orange', swatchColor: '#f97316' },
    { identifier: 'teal', label: 'Teal', swatchColor: '#14b8a6' },
    { identifier: 'emerald', label: 'Emerald', swatchColor: '#10b981' }
];

/**
 * Settings panel dedicated to theme modes, accent color palettes,
 * typography scale, and interface corner radius.
 *
 * @returns {JSX.Element} The rendered appearance settings panel.
 */
export const AppearancePanel: Component = () => {
    return (
        <div class="settings-panel-content appearance-panel">
            <h2 class="settings-panel-title">Appearance</h2>
            <p class="settings-panel-description">
                Personalize the interface theme, accent colors, typography, and geometry to suit
                your environment and creative workflow.
            </p>

            <SectionGroup
                title="Theme & Mode"
                description="Core visual theme and overall brightness of the interface."
            >
                <div class="appearance-setting-row">
                    <span class="setting-label">Interface Mode:</span>
                    <ToggleGroup
                        type="single"
                        value={appearance().mode}
                        onValueChange={selectedMode => {
                            if (selectedMode) {
                                appearanceActions.update({ mode: selectedMode as ThemeMode });
                            }
                        }}
                        size="sm"
                    >
                        <ToggleGroupItem value="light">
                            <Sun size={14} />
                            <span>Light</span>
                        </ToggleGroupItem>
                        <ToggleGroupItem value="dark">
                            <Moon size={14} />
                            <span>Dark</span>
                        </ToggleGroupItem>
                        <ToggleGroupItem value="system">
                            <Monitor size={14} />
                            <span>System</span>
                        </ToggleGroupItem>
                        <ToggleGroupItem value="custom">
                            <Palette size={14} />
                            <span>Custom</span>
                        </ToggleGroupItem>
                    </ToggleGroup>
                </div>

                <Show when={appearance().mode === 'custom'}>
                    <CustomThemeEditor />
                </Show>

                <div class="appearance-setting-row vertical">
                    <span class="setting-label">Accent Theme:</span>
                    <div class="appearance-accent-grid" role="radiogroup" aria-label="Accent theme">
                        <For each={ACCENT_THEME_OPTIONS}>
                            {accentThemeOption => {
                                const isSelected = () =>
                                    appearance().theme === accentThemeOption.identifier;
                                return (
                                    <button
                                        type="button"
                                        role="radio"
                                        aria-checked={isSelected()}
                                        class={`appearance-accent-button ${isSelected() ? 'is-active' : ''}`}
                                        onClick={() =>
                                            appearanceActions.update({
                                                theme: accentThemeOption.identifier
                                            })
                                        }
                                    >
                                        <span
                                            class="appearance-accent-dot"
                                            style={{
                                                'background-color': accentThemeOption.swatchColor
                                            }}
                                        />
                                        <span>{accentThemeOption.label}</span>
                                    </button>
                                );
                            }}
                        </For>
                    </div>
                </div>
            </SectionGroup>

            <SectionGroup
                title="Typography"
                description="Adjust text sizing for reading comfort and information density."
            >
                <div class="appearance-setting-row">
                    <span class="setting-label">Base Font Size:</span>
                    <ToggleGroup
                        type="single"
                        value={appearance().fontSize}
                        onValueChange={selectedFontSize => {
                            if (selectedFontSize) {
                                appearanceActions.update({
                                    fontSize: selectedFontSize as ThemeFontSize
                                });
                            }
                        }}
                        size="sm"
                    >
                        <ToggleGroupItem value="small">Small (11px)</ToggleGroupItem>
                        <ToggleGroupItem value="medium">Medium (13px)</ToggleGroupItem>
                        <ToggleGroupItem value="large">Large (15px)</ToggleGroupItem>
                    </ToggleGroup>
                </div>
            </SectionGroup>

            <SectionGroup
                title="Geometry"
                description="Control the shape and curvature of buttons, cards, and UI containers."
            >
                <div class="appearance-setting-row vertical">
                    <span class="setting-label">Corner Radius: {appearance().radius}px</span>
                    <div class="appearance-geometry-content">
                        <div class="appearance-slider-container">
                            <Slider
                                minimumValue={0}
                                maximumValue={16}
                                stepValue={2}
                                showTicks={true}
                                value={appearance().radius}
                                onValueChange={newRadius =>
                                    appearanceActions.update({ radius: newRadius })
                                }
                                showTooltip
                                formatValue={radiusValue => `${radiusValue}px`}
                            />
                            <div class="appearance-radius-preview-markers">
                                <span>Sharp (0px)</span>
                                <span>Default (6px)</span>
                                <span>Soft (16px)</span>
                            </div>
                        </div>
                        <div
                            class="appearance-radius-live-sample"
                            style={{ 'border-radius': `${appearance().radius}px` }}
                        >
                            <div
                                class="appearance-radius-sample-card"
                                style={{ 'border-radius': `${appearance().radius}px` }}
                            >
                                <div
                                    class="appearance-radius-sample-pill"
                                    style={{
                                        'border-radius': `${Math.max(2, appearance().radius - 2)}px`
                                    }}
                                >
                                    Button Preview
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </SectionGroup>
        </div>
    );
};
