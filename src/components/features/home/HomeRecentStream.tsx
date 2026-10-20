import { Component, For, Show, createSignal, onMount, createEffect } from 'solid-js';
import { Clock, ArrowRight, FolderSearch } from 'lucide-solid';
import { tagService } from '../../../lib/tags';
import { AssetItem } from '../../../types';
import { useSelection, useMetadata } from '../../../core/hooks';
import { filterActions } from '../../../core/store/filter';
import { formatFileSize, formatRelativeTime } from '../../../utils/format';
import { Thumbnail } from '../viewport/assets/Thumbnail';
import { HomeRecentStreamProperties, RecentAssetCardProperties } from './types';

/**
 * Individual card displaying a recently added or indexed asset.
 *
 * @param {RecentAssetCardProperties} properties - Component properties.
 * @returns {JSX.Element} The rendered asset card.
 */
const RecentAssetCard: Component<RecentAssetCardProperties> = properties => {
    return (
        <button
            type="button"
            class="home-recent-card"
            onClick={() => properties.onSelectAsset(properties.assetItem)}
            title={`Open ${properties.assetItem.filename} in Gallery`}
        >
            <div class="home-recent-thumbnail-container">
                <span class="home-recent-format-badge">{properties.assetItem.format}</span>
                <Thumbnail
                    id={properties.assetItem.id}
                    src={properties.assetItem.path}
                    thumbnail={properties.assetItem.thumbnail_path}
                    alt={properties.assetItem.filename}
                    width={properties.assetItem.width}
                    height={properties.assetItem.height}
                    mediaType={properties.assetItem.media_type}
                    state={properties.assetItem.state}
                />
            </div>
            <div class="home-recent-card-body">
                <p class="home-recent-card-name" title={properties.assetItem.filename}>
                    {properties.assetItem.filename}
                </p>
                <div class="home-recent-card-footer">
                    <span>{formatFileSize(properties.assetItem.size)}</span>
                    <span>{formatRelativeTime(properties.assetItem.added_at)}</span>
                </div>
            </div>
        </button>
    );
};

/**
 * Component displaying the stream of most recently added or indexed assets.
 * Allows quick exploration of newly discovered files and immediate navigation to the gallery.
 *
 * @param {HomeRecentStreamProperties} properties - Component properties.
 * @returns {JSX.Element} The rendered recent assets stream.
 */
export const HomeRecentStream: Component<HomeRecentStreamProperties> = properties => {
    const selection = useSelection();
    const metadata = useMetadata();
    const [recentAssets, setRecentAssets] = createSignal<AssetItem[]>([]);
    const [isLoading, setIsLoading] = createSignal<boolean>(true);

    /**
     * Loads the most recently added or indexed assets from the database.
     *
     * @returns {Promise<void>}
     */
    const loadRecentAssets = async (): Promise<void> => {
        setIsLoading(true);
        try {
            const paginatedResult = await tagService.getAssets(
                {
                    sortBy: 'added_at',
                    sortOrder: 'desc'
                },
                1,
                12
            );
            setRecentAssets(paginatedResult.items);
        } catch (fetchError) {
            console.error('Failed to load recent assets:', fetchError);
        } finally {
            setIsLoading(false);
        }
    };

    onMount(() => {
        loadRecentAssets();
    });

    // Re-fetch when metadata tag update version changes
    createEffect(() => {
        if (metadata.tagUpdateVersion > 0) {
            loadRecentAssets();
        }
    });

    /**
     * Handles selecting an asset and navigating to the Gallery with that asset focused.
     *
     * @param {AssetItem} selectedAsset - The clicked asset.
     * @returns {void}
     */
    const handleSelectAsset = (selectedAsset: AssetItem): void => {
        selection.select([selectedAsset.id]);
        properties.onNavigate('gallery');
    };

    /**
     * Handles navigating to the Gallery with sorting configured for latest additions.
     *
     * @returns {void}
     */
    const handleViewAllRecent = (): void => {
        filterActions.clearAll();
        filterActions.setSortBy('added_at');
        filterActions.setSortOrder('desc');
        properties.onNavigate('gallery');
    };

    return (
        <section class="home-recent-stream" aria-label="Recently Added Assets">
            <div class="home-section-header">
                <div class="home-section-title-group">
                    <h2>Recently Added & Indexed</h2>
                    <p>Latest visual and multimedia references ingested into your library</p>
                </div>

                <Show when={recentAssets().length > 0}>
                    <button
                        type="button"
                        class="home-section-action-btn"
                        onClick={handleViewAllRecent}
                    >
                        <span>View all in Gallery</span>
                        <ArrowRight size={14} />
                    </button>
                </Show>
            </div>

            <Show
                when={!isLoading() && recentAssets().length > 0}
                fallback={
                    <Show
                        when={!isLoading()}
                        fallback={
                            <div class="home-empty-state">
                                <Clock size={32} class="home-empty-state-icon" />
                                <h3>Loading recent assets...</h3>
                            </div>
                        }
                    >
                        <div class="home-empty-state">
                            <FolderSearch size={36} class="home-empty-state-icon" />
                            <h3>No indexed assets found</h3>
                            <p>
                                Add a library folder in Settings or wait for initial folder indexing
                                to populate your library stream.
                            </p>
                        </div>
                    </Show>
                }
            >
                <div class="home-recent-grid">
                    <For each={recentAssets()}>
                        {assetItem => (
                            <RecentAssetCard
                                assetItem={assetItem}
                                onSelectAsset={handleSelectAsset}
                            />
                        )}
                    </For>
                </div>
            </Show>
        </section>
    );
};
