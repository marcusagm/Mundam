import { Component, JSX, Show } from 'solid-js';
import { Thumbnail } from './Thumbnail';
import { Heart, CheckCircle2, ArchiveRestore } from 'lucide-solid';
import { AssetItemContainer } from './AssetItemContainer';
import { AssetCardOverlay } from './AssetCardOverlay';
import { AssetCardStacked } from './AssetCardStacked';
import { AssetItem } from '../../../../types';
import { useViewportPreferences } from '../../../../core/hooks/useViewportPreferences';
import { filterState } from '../../../../core/store/filter';
import { itemActions } from '../../../../core/store/library/itemActions';
import './asset-card.css';

/**
 * AssetCardProps - Re-defined to consume the full AssetItem
 */
export interface AssetCardProps {
    /**
     * Full asset item data
     * @type {AssetItem}
     */
    item: AssetItem;

    /**
     * State (controlled externally)
     * @type {boolean}
     */
    isSelected: boolean;
    /**
     * Whether it has keyboard focus
     * @type {boolean | undefined}
     */
    isFocused?: boolean;
    /**
     * Positioning style for virtualization
     * @type {JSX.CSSProperties}
     */
    style: JSX.CSSProperties;
    /**
     * Optional Extra Wrapper Class
     * @type {string | undefined}
     */
    className?: string;

    /**
     * Callbacks (lifted to parent)
     */
    onSelect: (id: string, modifiers: { multi: boolean; shift: boolean }) => void;
    onOpen: (id: string) => void;
    onContextMenu?: (event: MouseEvent, id: string) => void;

    /**
     * DnD Support
     */
    getSelectedIds: () => string[];
    getItemInfo: (id: string) => { path: string; thumbnail_path: string | null } | undefined;
}

/**
 * AssetCard - Main Presentational Facade Component
 *
 * Wraps the abstract drag container and renders either the Overlay
 * or Stacked variants of metadata display, based on user preferences.
 *
 * @param {AssetCardProps} props - The complete asset card parameters.
 * @returns {JSX.Element} The visual composition.
 */
export const AssetCard: Component<AssetCardProps> = (props: AssetCardProps): JSX.Element => {
    /**
     * Viewport preferences store
     */
    const preferences = useViewportPreferences();

    return (
        <AssetItemContainer
            id={props.item.id}
            path={props.item.path}
            thumbnailPath={props.item.thumbnail_path}
            filename={props.item.filename}
            isSelected={props.isSelected}
            isFocused={props.isFocused}
            style={props.style}
            class={props.className}
            onSelect={props.onSelect}
            onOpen={props.onOpen}
            onContextMenu={props.onContextMenu}
            getSelectedIds={props.getSelectedIds}
            getItemInfo={props.getItemInfo}
        >
            {() => (
                <div class="asset-card-content">
                    <div class="asset-card-media-wrapper">
                        <Thumbnail
                            id={props.item.id}
                            src={props.item.path}
                            thumbnail={props.item.thumbnail_path}
                            alt={props.item.filename}
                            width={props.item.width}
                            height={props.item.height}
                            mediaType={props.item.media_type}
                            state={props.item.state}
                        />

                        {/* Top Right Overlay for Selection and Favorites */}
                        <div class="asset-card-actions">
                            <Show when={props.isSelected}>
                                <div class="asset-card-selection-icon">
                                    <CheckCircle2
                                        size={20}
                                        fill="currentColor"
                                        color="var(--bg-secondary)"
                                    />
                                </div>
                            </Show>
                            <Show when={filterState.filterTrash}>
                                <div
                                    class="asset-card-restore-icon"
                                    onClick={event => {
                                        event.stopPropagation();
                                        itemActions.restoreFromTrashAssets([props.item.id]);
                                    }}
                                    title="Restore from trash"
                                >
                                    <ArchiveRestore size={20} />
                                </div>
                            </Show>
                            <div
                                class={`asset-card-favorite-icon ${props.item.is_favorite ? 'is-favorite' : ''}`}
                                onClick={event => {
                                    event.stopPropagation();
                                    itemActions.toggleItemFavorite(props.item.id);
                                }}
                            >
                                <Heart
                                    size={20}
                                    fill={props.item.is_favorite ? 'currentColor' : 'none'}
                                />
                            </div>
                        </div>

                        {/* Metadata Overlay Component */}
                        <Show when={preferences.metadataPosition === 'overlay'}>
                            <AssetCardOverlay
                                item={props.item}
                                visibleFields={preferences.visibleFields}
                            />
                        </Show>
                    </div>

                    {/* Metadata Stacked Component */}
                    <Show when={preferences.metadataPosition === 'stacked'}>
                        <AssetCardStacked
                            item={props.item}
                            visibleFields={preferences.visibleFields}
                        />
                    </Show>
                </div>
            )}
        </AssetItemContainer>
    );
};
