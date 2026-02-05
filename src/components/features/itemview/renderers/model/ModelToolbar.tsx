import { Component } from 'solid-js';
import { RotateCw, Grid, RefreshCcw } from 'lucide-solid';
import { Button } from '../../../../ui/Button';
import { ToggleGroup, ToggleGroupItem } from '../../../../ui/ToggleGroup';
import { Tooltip } from '../../../../ui/Tooltip';
import { useItemViewContext, ModelSettings } from '../../ItemViewContext';
import { ColorPicker, Popover } from '../../../../ui';

export const ModelToolbar: Component = () => {
    const { modelSettings, setModelSettings, reset } = useItemViewContext();

    const toggleAutoRotate = () => {
        setModelSettings((s: ModelSettings) => ({ ...s, autoRotate: !s.autoRotate }));
    };

    const toggleGrid = () => {
        setModelSettings((s: ModelSettings) => ({ ...s, showGrid: !s.showGrid }));
    };

    const setBg = (color: string) => {
        setModelSettings((s: ModelSettings) => ({ ...s, backgroundColor: color }));
    };

    return (
        <>
            <div class="toolbar-group">
                <div class="toolbar-label">View</div>

                <Tooltip position="bottom" content="Reset View">
                    <Button variant="ghost" size="icon" onClick={() => reset()}>
                        <RefreshCcw size={16} />
                    </Button>
                </Tooltip>

                <div class="toolbar-separator" />

                <ToggleGroup
                    type="multiple"
                    value={[
                        ...(modelSettings().autoRotate ? ['autorotate'] : []),
                        ...(modelSettings().showGrid ? ['grid'] : [])
                    ]}
                >
                    <Tooltip position="bottom" content="Auto Rotate">
                        <ToggleGroupItem value="autorotate" onClick={toggleAutoRotate}>
                            <RotateCw size={16} />
                        </ToggleGroupItem>
                    </Tooltip>

                    <Tooltip position="bottom" content="Show Grid">
                        <ToggleGroupItem value="grid" onClick={toggleGrid}>
                            <Grid size={16} />
                        </ToggleGroupItem>
                    </Tooltip>
                </ToggleGroup>
            </div>

            <div class="toolbar-group">
                <div style={{ display: 'flex', gap: '4px' }}>
                    <Popover
                        trigger={
                            <Button variant="ghost" class="model-bg-color-btn">
                                <div
                                    class="model-bg-color-preview"
                                    style={{
                                        background: modelSettings().backgroundColor
                                    }}
                                />
                                Background 2
                            </Button>
                        }
                    >
                        <div class="model-bg-color-popover">
                            <ColorPicker
                                color={modelSettings().backgroundColor}
                                onChange={c => setBg(c)}
                                allowNoColor
                            />
                        </div>
                    </Popover>
                </div>
            </div>

            <style>{`
                .color-swatch {
                    width: 20px;
                    height: 20px;
                    border-radius: 50%;
                    border: 1px solid var(--border-color);
                    cursor: pointer;
                    transition: all 0.2s;
                    padding: 0;
                }
                .color-swatch:hover {
                    transform: scale(1.1);
                    border-color: var(--text-primary);
                }
                .color-swatch.active {
                    border-color: var(--accent-color);
                    box-shadow: 0 0 0 2px var(--bg-surface), 0 0 0 4px var(--accent-color);
                }
            `}</style>
        </>
    );
};
