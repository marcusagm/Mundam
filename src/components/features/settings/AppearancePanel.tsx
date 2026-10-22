import { Component, Show } from 'solid-js';
import { CustomThemeEditor } from './CustomThemeEditor';
import { SectionGroup } from '../../ui/SectionGroup';
import { RadioGroup, RadioGroupItem } from '../../ui/RadioGroup';
import { Select } from '../../ui/Select';
import { Slider } from '../../ui/Slider';
import {
    appearance,
    appearanceActions,
    ThemeColor,
    ThemeFontSize,
    ThemeMode
} from '../../../core/store/appearanceStore';
import './appearance-panel.css';

export const AppearancePanel: Component = () => {
    const themeOptions = [
        { value: 'neutral', label: 'Neutral (Default)' },
        { value: 'zinc', label: 'Zinc' },
        { value: 'slate', label: 'Slate' },
        { value: 'stone', label: 'Stone' },
        { value: 'blue', label: 'Blue' },
        { value: 'indigo', label: 'Indigo' },
        { value: 'violet', label: 'Violet' },
        { value: 'fuchsia', label: 'Fuchsia' },
        { value: 'rose', label: 'Rose' },
        { value: 'orange', label: 'Orange' },
        { value: 'teal', label: 'Teal' },
        { value: 'emerald', label: 'Emerald' }
    ];

    return (
        <div class="settings-panel-content appearance-panel">
            <h2 class="settings-panel-title">Appearance</h2>
            <p class="settings-panel-description">
                Maximize intermediate creative focus by customizing the look of Mundam.
            </p>

            <SectionGroup
                title="Theme & Mode"
                description="Core visual settings for the interface."
            >
                <div class="appearance-setting-row">
                    <span class="setting-label">Interface Mode:</span>
                    <RadioGroup
                        value={appearance().mode}
                        onValueChange={v => appearanceActions.update({ mode: v as ThemeMode })}
                        orientation="horizontal"
                    >
                        <RadioGroupItem value="light" label="Light" />
                        <RadioGroupItem value="dark" label="Dark" />
                        <RadioGroupItem value="system" label="System" />
                        <RadioGroupItem value="custom" label="Custom" />
                    </RadioGroup>
                </div>

                <Show when={appearance().mode === 'custom'}>
                    <CustomThemeEditor />
                </Show>

                <div class="appearance-setting-row">
                    <span class="setting-label">Accent Theme:</span>
                    <div style={{ width: '220px' }}>
                        <Select
                            options={themeOptions}
                            value={appearance().theme}
                            onValueChange={v =>
                                appearanceActions.update({ theme: v as ThemeColor })
                            }
                            size="sm"
                        />
                    </div>
                </div>
            </SectionGroup>

            <SectionGroup
                title="Typography"
                description="Adjust text sizing for comfort and information density."
            >
                <div class="appearance-setting-row">
                    <span class="setting-label">Base Font Size:</span>
                    <RadioGroup
                        value={appearance().fontSize}
                        onValueChange={v =>
                            appearanceActions.update({ fontSize: v as ThemeFontSize })
                        }
                        orientation="horizontal"
                    >
                        <RadioGroupItem value="small" label="Small" />
                        <RadioGroupItem value="medium" label="Medium" />
                        <RadioGroupItem value="large" label="Large" />
                    </RadioGroup>
                </div>
            </SectionGroup>

            <SectionGroup
                title="Geometry"
                description="Control the shape and curvature of UI elements."
            >
                <div class="appearance-setting-row vertical">
                    <span class="setting-label">Corner Radius:</span>
                    <div style={{ padding: 'var(--p-space-xs) var(--p-space-s)' }}>
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
                            formatValue={value => `${value}px`}
                        />
                        <div class="appearance-radius-preview">
                            <span>Sharp (0px)</span>
                            <span>Default (6px)</span>
                            <span>Soft (16px)</span>
                        </div>
                    </div>
                </div>
            </SectionGroup>
        </div>
    );
};
