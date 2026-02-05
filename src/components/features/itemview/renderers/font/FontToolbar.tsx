import { Component } from 'solid-js';
import { Type, AlignJustify, MoveVertical } from 'lucide-solid';
import { Popover } from '../../../../ui/Popover';
import { ColorPicker } from '../../../../ui/ColorPicker';
import { Slider } from '../../../../ui/Slider';
import { Button } from '../../../../ui/Button';
import { useItemViewContext, FontSettings } from '../../ItemViewContext';
import { Tooltip } from '../../../../ui/Tooltip';
import './font-view.css';

export const FontToolbar: Component = () => {
    const { fontSettings, setFontSettings } = useItemViewContext();

    const updateSetting = (key: keyof FontSettings, value: any) => {
        setFontSettings(prev => ({ ...prev, [key]: value }));
    };

    return (
        <>
            <div class="toolbar-group">
                <Tooltip position="bottom" content="Font Size">
                    <div class="font-control-group">
                        <Type size={14} class="font-icon" />
                        <div style={{ width: '80px' }}>
                            <Slider
                                min={8}
                                max={200}
                                value={fontSettings().fontSize}
                                onValueChange={val => updateSetting('fontSize', val)}
                            />
                        </div>
                        <span class="font-control-value">
                            {Math.round(fontSettings().fontSize)}
                        </span>
                    </div>
                </Tooltip>

                <div class="toolbar-separator" />

                <Tooltip position="bottom" content="Line Height">
                    <div class="font-control-group">
                        <MoveVertical size={14} class="font-icon" />
                        <div style={{ width: '80px' }}>
                            <Slider
                                min={0.5}
                                max={3}
                                step={0.1}
                                value={fontSettings().lineHeight}
                                onValueChange={val => updateSetting('lineHeight', val)}
                                showTicks={false}
                            />
                        </div>
                    </div>
                </Tooltip>

                <Tooltip position="bottom" content="Letter Spacing">
                    <div class="font-control-group">
                        <AlignJustify
                            size={14}
                            class="font-icon"
                            style={{ transform: 'rotate(90deg)' }}
                        />
                        <div style={{ width: '80px' }}>
                            <Slider
                                min={-5}
                                max={20}
                                value={fontSettings().letterSpacing}
                                onValueChange={val => updateSetting('letterSpacing', val)}
                            />
                        </div>
                    </div>
                </Tooltip>
            </div>

            <div class="toolbar-group">
                <Popover
                    trigger={
                        <Button variant="ghost" class="font-color-btn">
                            <div
                                class="font-color-preview"
                                style={{ background: fontSettings().color }}
                            />
                            Text Color
                        </Button>
                    }
                >
                    <div class="font-color-popover">
                        <ColorPicker
                            color={fontSettings().color}
                            onChange={c => updateSetting('color', c)}
                        />
                    </div>
                </Popover>

                <Popover
                    trigger={
                        <Button variant="ghost" class="font-color-btn">
                            <div
                                class="font-color-preview"
                                style={{
                                    background: fontSettings().backgroundColor,
                                    'border-radius': '2px'
                                }}
                            />
                            Bg Color
                        </Button>
                    }
                >
                    <div class="font-color-popover">
                        <ColorPicker
                            color={fontSettings().backgroundColor}
                            onChange={c => updateSetting('backgroundColor', c)}
                            allowNoColor
                        />
                    </div>
                </Popover>
            </div>
        </>
    );
};
