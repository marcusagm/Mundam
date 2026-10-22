import { Component, For, JSX, Show } from 'solid-js';
import {
    Button,
    PopoverRoot,
    PopoverTrigger,
    PopoverContent,
    Slider,
    ToggleGroup,
    ToggleGroupItem,
    Tooltip,
    Checkbox
} from '../../../ui';
import {
    LayoutGrid,
    AlignCenterVertical,
    AlignCenterHorizontal,
    List,
    PanelTop,
    PanelBottom
} from 'lucide-solid';
import { useFilters, useViewportPreferences } from '../../../../core/hooks';
import {
    TOGGLEABLE_LIST_COLUMNS,
    isListColumnHidden,
    updateListColumnConfig
} from '../../../../core/store/listColumnsStore';
import { type MetadataField } from '../../../../core/store/viewportPreferencesStore';

/**
 * Available metadata fields for display
 */
const AVAILABLE_FIELDS: { value: MetadataField; label: string }[] = [
    { value: 'filename', label: 'File Name' },
    { value: 'extension', label: 'Extension' },
    { value: 'dimensions', label: 'Dimensions' },
    { value: 'size', label: 'File Size' },
    { value: 'rating', label: 'Rating' },
    { value: 'modified_at', label: 'Date Modified' },
    { value: 'created_at', label: 'Date Created' },
    { value: 'added_at', label: 'Date Added' },
    { value: 'tags', label: 'Tags' }
];

/**
 * Component responsible for selecting layout mode, thumbnail size,
 * metadata positioning, and visible metadata fields.
 * Uses the filters store and viewport preferences store.
 *
 * @returns {JSX.Element} The view configuration popover.
 */
export const ViewConfiguration: Component = (): JSX.Element => {
    /**
     * Filters store
     */
    const filters = useFilters();

    /**
     * Viewport preferences store
     */
    const preferences = useViewportPreferences();

    return (
        <PopoverRoot placement="bottom-end">
            <PopoverTrigger>
                <Tooltip content="View Options" placement="bottom">
                    <Button variant="ghost" size="icon">
                        <LayoutGrid size={18} />
                    </Button>
                </Tooltip>
            </PopoverTrigger>
            <PopoverContent class="toolbar-popover-view-configuration">
                <div class="popover-config-group">
                    <span class="popover-config-label">Layout Mode</span>
                    <ToggleGroup
                        type="single"
                        value={filters.layout}
                        onValueChange={(val: string) =>
                            val &&
                            filters.setLayout(val as 'grid' | 'list' | 'masonry-v' | 'masonry-h')
                        }
                    >
                        <Tooltip content="Masonry Vertical" placement="top">
                            <ToggleGroupItem value="masonry-v">
                                <AlignCenterVertical size={16} />
                            </ToggleGroupItem>
                        </Tooltip>
                        <Tooltip content="Masonry Horizontal" placement="top">
                            <ToggleGroupItem value="masonry-h">
                                <AlignCenterHorizontal size={16} />
                            </ToggleGroupItem>
                        </Tooltip>
                        <Tooltip content="Grid" placement="top">
                            <ToggleGroupItem value="grid">
                                <LayoutGrid size={16} />
                            </ToggleGroupItem>
                        </Tooltip>
                        <Tooltip content="List" placement="top">
                            <ToggleGroupItem value="list">
                                <List size={16} />
                            </ToggleGroupItem>
                        </Tooltip>
                    </ToggleGroup>
                </div>
                <div class="popover-config-group" style={{ 'margin-top': '16px' }}>
                    <span class="popover-config-label">Thumbnail Size</span>
                    <div style={{ 'padding-top': '8px' }}>
                        <Slider
                            value={filters.thumbSize || 200}
                            minimumValue={100}
                            maximumValue={500}
                            showTicks={false}
                            onValueChange={(newThumbnailSize: number | number[]) => {
                                const val = Array.isArray(newThumbnailSize)
                                    ? newThumbnailSize[0]
                                    : newThumbnailSize;
                                filters.setThumbSize(val);
                            }}
                            title="Thumbnail Size"
                        />
                    </div>
                </div>
                <Show
                    when={filters.layout !== 'list'}
                    fallback={
                        <div class="popover-config-group" style={{ 'margin-top': '16px' }}>
                            <span class="popover-config-label">Visible Columns</span>
                            <div
                                class="popover-config-fields-list"
                                style={{
                                    display: 'flex',
                                    'flex-direction': 'column',
                                    gap: '8px',
                                    'margin-top': '8px'
                                }}
                            >
                                <For each={TOGGLEABLE_LIST_COLUMNS}>
                                    {column => (
                                        <Checkbox
                                            label={column.label}
                                            checked={
                                                !isListColumnHidden(
                                                    column.key,
                                                    column.defaultHidden
                                                )
                                            }
                                            onCheckedChange={() =>
                                                updateListColumnConfig(column.key, {
                                                    hidden: !isListColumnHidden(
                                                        column.key,
                                                        column.defaultHidden
                                                    )
                                                })
                                            }
                                        />
                                    )}
                                </For>
                            </div>
                        </div>
                    }
                >
                    <div class="popover-config-group" style={{ 'margin-top': '16px' }}>
                        <span class="popover-config-label">Metadata Position</span>
                        <ToggleGroup
                            type="single"
                            value={preferences.metadataPosition}
                            onValueChange={(val: string) =>
                                val && preferences.setMetadataPosition(val as 'overlay' | 'stacked')
                            }
                        >
                            <Tooltip content="Overlay (On Hover)" placement="top">
                                <ToggleGroupItem value="overlay" style={{ flex: 1 }}>
                                    <div
                                        style={{
                                            display: 'flex',
                                            'align-items': 'center',
                                            gap: '8px'
                                        }}
                                    >
                                        <PanelTop size={14} />
                                        <span>Overlay</span>
                                    </div>
                                </ToggleGroupItem>
                            </Tooltip>
                            <Tooltip content="Stacked (Below Thumbnail)" placement="top">
                                <ToggleGroupItem value="stacked" style={{ flex: 1 }}>
                                    <div
                                        style={{
                                            display: 'flex',
                                            'align-items': 'center',
                                            gap: '8px'
                                        }}
                                    >
                                        <PanelBottom size={14} />
                                        <span>Stacked</span>
                                    </div>
                                </ToggleGroupItem>
                            </Tooltip>
                        </ToggleGroup>
                    </div>
                    <div class="popover-config-group" style={{ 'margin-top': '16px' }}>
                        <span class="popover-config-label">Visible Metadata Fields</span>
                        <div
                            class="popover-config-fields-list"
                            style={{
                                display: 'flex',
                                'flex-direction': 'column',
                                gap: '8px',
                                'margin-top': '8px'
                            }}
                        >
                            <For each={AVAILABLE_FIELDS}>
                                {field => (
                                    <Checkbox
                                        label={field.label}
                                        checked={preferences.visibleFields.includes(field.value)}
                                        onCheckedChange={() =>
                                            preferences.toggleVisibleField(field.value)
                                        }
                                    />
                                )}
                            </For>
                        </div>
                    </div>
                </Show>
            </PopoverContent>
        </PopoverRoot>
    );
};
