import { Component, onMount } from 'solid-js';
import { Layers, BookmarkX, BookmarkCheck, Heart, Trash2 } from 'lucide-solid';
import { useMetadata, useFilters } from '../../../core/hooks';
import { CountBadge } from '../../ui/CountBadge';
import { SidebarPanel } from '../../ui/SidebarPanel';

export const LibrarySidebarPanel: Component = () => {
    const metadata = useMetadata();
    const filters = useFilters();

    onMount(async () => {
        await metadata.loadStats();
    });

    const isAllItemsActive = () =>
        !filters.selectedFolderId &&
        !filters.filterUntagged &&
        !filters.filterHasTags &&
        !filters.filterFavorites &&
        !filters.filterTrash &&
        filters.selectedTags.length === 0;

    return (
        <SidebarPanel title="Library" class="panel-fixed">
            <button
                type="button"
                class={`nav-item ${isAllItemsActive() ? 'active' : ''}`}
                aria-pressed={isAllItemsActive()}
                onClick={() => filters.clearAll()}
            >
                <Layers size={16} />
                <span style={{ flex: 1 }}>All Items</span>
                <CountBadge count={metadata.stats.total_assets} variant="secondary" />
            </button>
            <button
                type="button"
                class={`nav-item ${filters.filterUntagged ? 'active' : ''}`}
                aria-pressed={filters.filterUntagged}
                onClick={event => filters.toggleUntagged(event.ctrlKey || event.metaKey)}
            >
                <BookmarkX size={16} />
                <span style={{ flex: 1 }}>Untagged</span>
                <CountBadge
                    count={metadata.stats.untagged_assets || 0}
                    variant="secondary"
                    showZero={true}
                />
            </button>
            <button
                type="button"
                class={`nav-item ${filters.filterHasTags ? 'active' : ''}`}
                aria-pressed={filters.filterHasTags}
                onClick={event => filters.toggleHasTags(event.ctrlKey || event.metaKey)}
            >
                <BookmarkCheck size={16} />
                <span style={{ flex: 1 }}>Has Tags</span>
                <CountBadge
                    count={metadata.stats.has_tags_assets || 0}
                    variant="secondary"
                    showZero={true}
                />
            </button>
            <button
                type="button"
                class={`nav-item ${filters.filterFavorites ? 'active' : ''}`}
                aria-pressed={filters.filterFavorites}
                onClick={event => filters.toggleFavorites(event.ctrlKey || event.metaKey)}
            >
                <Heart size={16} />
                <span style={{ flex: 1 }}>Favorites</span>
                <CountBadge
                    count={metadata.stats.favorite_assets || 0}
                    variant="secondary"
                    showZero={true}
                />
            </button>
            <button
                type="button"
                class={`nav-item ${filters.filterTrash ? 'active' : ''}`}
                aria-pressed={filters.filterTrash}
                onClick={event => filters.toggleTrash(event.ctrlKey || event.metaKey)}
            >
                <Trash2 size={16} />
                <span style={{ flex: 1 }}>Trash</span>
                <CountBadge
                    count={metadata.stats.trash_assets || 0}
                    variant="secondary"
                    showZero={true}
                />
            </button>
        </SidebarPanel>
    );
};
