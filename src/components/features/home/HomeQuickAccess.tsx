import { Component } from 'solid-js';
import {
    Files,
    FolderTree,
    Tags,
    Tag,
    BookmarkX,
    SlidersHorizontal,
    Copy,
    Star,
    Trash2
} from 'lucide-solid';
import { useMetadata } from '../../../core/hooks';
import { filterActions } from '../../../core/store/filter';
import { formatCompactNumber, formatFileSize } from '../../../utils/format';
import { HomeQuickAccessProperties } from './types';

/**
 * Quick access dashboard component displaying core library counters.
 * Provides high-visibility metrics for library size, folder indexation, tag taxonomy,
 * smart folders, duplicates, favorites, and trash, serving as interactive teleport shortcuts.
 *
 * @param {HomeQuickAccessProperties} properties - Component properties.
 * @returns {JSX.Element} The rendered quick access counters grid.
 */
export const HomeQuickAccess: Component<HomeQuickAccessProperties> = properties => {
    const metadata = useMetadata();

    /**
     * Handles navigation to the general Gallery view without filters.
     *
     * @returns {void}
     */
    const handleNavigateToAllAssets = (): void => {
        filterActions.clearAll();
        properties.onNavigate('gallery');
    };

    /**
     * Handles navigation to the Gallery with the Tagged filter activated.
     *
     * @returns {void}
     */
    const handleNavigateToTagged = (): void => {
        filterActions.clearAll();
        filterActions.setHasTags(true);
        properties.onNavigate('gallery');
    };

    /**
     * Handles navigation to the Gallery with the Untagged filter activated.
     *
     * @returns {void}
     */
    const handleNavigateToUntagged = (): void => {
        filterActions.clearAll();
        filterActions.setUntagged(true);
        properties.onNavigate('gallery');
    };

    /**
     * Handles navigation to the Duplicate Finder view.
     *
     * @returns {void}
     */
    const handleNavigateToDuplicates = (): void => {
        properties.onNavigate('duplicates');
    };

    /**
     * Handles navigation to the Gallery with the Favorites filter activated.
     *
     * @returns {void}
     */
    const handleNavigateToFavorites = (): void => {
        filterActions.clearAll();
        filterActions.setFavorites(true);
        properties.onNavigate('gallery');
    };

    /**
     * Handles navigation to the Gallery with the Trash filter activated.
     *
     * @returns {void}
     */
    const handleNavigateToTrash = (): void => {
        filterActions.clearAll();
        filterActions.setTrash(true);
        properties.onNavigate('gallery');
    };

    return (
        <section class="home-quick-access" aria-label="Library Quick Access Counters">
            <div class="home-quick-access-grid">
                {/* 1. Total Assets */}
                <button
                    type="button"
                    class="home-quick-card"
                    onClick={handleNavigateToAllAssets}
                    title="View all assets in Gallery"
                >
                    <div
                        class="home-quick-card-icon"
                        style={{
                            background: 'oklch(58% var(--theme-c) var(--theme-h) / 0.15)',
                            color: 'var(--action-primary-bg)'
                        }}
                    >
                        <Files size={20} />
                    </div>
                    <div class="home-quick-card-info">
                        <span class="home-quick-card-value">
                            {formatCompactNumber(metadata.stats.total_assets)}
                        </span>
                        <span class="home-quick-card-label">Total Assets</span>
                        <span class="home-quick-card-subtext">
                            {formatFileSize(metadata.stats.total_size_bytes)} on disk
                        </span>
                    </div>
                </button>

                {/* 2. Indexed Folders */}
                <button
                    type="button"
                    class="home-quick-card"
                    onClick={handleNavigateToAllAssets}
                    title="View monitored directories"
                >
                    <div
                        class="home-quick-card-icon"
                        style={{
                            background: 'oklch(65% 0.15 200 / 0.15)',
                            color: 'oklch(68% 0.14 200)'
                        }}
                    >
                        <FolderTree size={20} />
                    </div>
                    <div class="home-quick-card-info">
                        <span class="home-quick-card-value">
                            {formatCompactNumber(metadata.stats.total_folders)}
                        </span>
                        <span class="home-quick-card-label">Indexed Folders</span>
                        <span class="home-quick-card-subtext">Watched locations</span>
                    </div>
                </button>

                {/* 3. Total Tags */}
                <button
                    type="button"
                    class="home-quick-card"
                    onClick={handleNavigateToAllAssets}
                    title="View tags and taxonomy"
                >
                    <div
                        class="home-quick-card-icon"
                        style={{
                            background: 'oklch(68% 0.18 310 / 0.15)',
                            color: 'oklch(68% 0.18 310)'
                        }}
                    >
                        <Tags size={20} />
                    </div>
                    <div class="home-quick-card-info">
                        <span class="home-quick-card-value">
                            {formatCompactNumber(metadata.stats.total_tags)}
                        </span>
                        <span class="home-quick-card-label">Total Tags</span>
                        <span class="home-quick-card-subtext">Taxonomy labels</span>
                    </div>
                </button>

                {/* 4. With Tags */}
                <button
                    type="button"
                    class="home-quick-card"
                    onClick={handleNavigateToTagged}
                    title="View assets with tags"
                >
                    <div
                        class="home-quick-card-icon"
                        style={{
                            background: 'oklch(65% 0.18 145 / 0.15)',
                            color: 'oklch(70% 0.15 145)'
                        }}
                    >
                        <Tag size={20} />
                    </div>
                    <div class="home-quick-card-info">
                        <span class="home-quick-card-value">
                            {formatCompactNumber(metadata.stats.has_tags_assets)}
                        </span>
                        <span class="home-quick-card-label">With Tags</span>
                        <span class="home-quick-card-subtext">Classified items</span>
                    </div>
                </button>

                {/* 5. Without Tags */}
                <button
                    type="button"
                    class="home-quick-card"
                    onClick={handleNavigateToUntagged}
                    title="View untagged assets to organize"
                >
                    <div
                        class="home-quick-card-icon"
                        style={{
                            background: 'oklch(75% 0.15 85 / 0.15)',
                            color: 'oklch(80% 0.13 85)'
                        }}
                    >
                        <BookmarkX size={20} />
                    </div>
                    <div class="home-quick-card-info">
                        <span class="home-quick-card-value">
                            {formatCompactNumber(metadata.stats.untagged_assets)}
                        </span>
                        <span class="home-quick-card-label">Without Tags</span>
                        <span class="home-quick-card-subtext">Pending triage</span>
                    </div>
                </button>

                {/* 6. Smart Folders */}
                <button
                    type="button"
                    class="home-quick-card"
                    onClick={handleNavigateToAllAssets}
                    title="View smart search rules"
                >
                    <div
                        class="home-quick-card-icon"
                        style={{
                            background: 'oklch(65% 0.16 260 / 0.15)',
                            color: 'oklch(71% 0.14 260)'
                        }}
                    >
                        <SlidersHorizontal size={20} />
                    </div>
                    <div class="home-quick-card-info">
                        <span class="home-quick-card-value">
                            {formatCompactNumber(metadata.stats.smart_folders)}
                        </span>
                        <span class="home-quick-card-label">Smart Folders</span>
                        <span class="home-quick-card-subtext">Saved query presets</span>
                    </div>
                </button>

                {/* 7. Duplicate Assets */}
                <button
                    type="button"
                    class="home-quick-card"
                    classList={{
                        'home-quick-card--alert': metadata.stats.duplicate_assets > 0
                    }}
                    onClick={handleNavigateToDuplicates}
                    title="Open Duplicate Finder view"
                >
                    <div
                        class="home-quick-card-icon"
                        style={{
                            background: 'oklch(70% 0.18 45 / 0.15)',
                            color: 'oklch(75% 0.16 45)'
                        }}
                    >
                        <Copy size={20} />
                    </div>
                    <div class="home-quick-card-info">
                        <span class="home-quick-card-value">
                            {formatCompactNumber(metadata.stats.duplicate_assets)}
                        </span>
                        <span class="home-quick-card-label">Duplicates</span>
                        <span class="home-quick-card-subtext">Candidate copies</span>
                    </div>
                </button>

                {/* 8. Favorite Assets */}
                <button
                    type="button"
                    class="home-quick-card"
                    onClick={handleNavigateToFavorites}
                    title="View favorite assets in Gallery"
                >
                    <div
                        class="home-quick-card-icon"
                        style={{
                            background: 'oklch(80% 0.15 90 / 0.15)',
                            color: 'oklch(82% 0.14 90)'
                        }}
                    >
                        <Star size={20} />
                    </div>
                    <div class="home-quick-card-info">
                        <span class="home-quick-card-value">
                            {formatCompactNumber(metadata.stats.favorite_assets)}
                        </span>
                        <span class="home-quick-card-label">Favorites</span>
                        <span class="home-quick-card-subtext">Starred references</span>
                    </div>
                </button>

                {/* 9. Trash Assets */}
                <button
                    type="button"
                    class="home-quick-card"
                    onClick={handleNavigateToTrash}
                    title="View assets in trash"
                >
                    <div
                        class="home-quick-card-icon"
                        style={{
                            background: 'oklch(60% 0.20 25 / 0.15)',
                            color: 'var(--text-destructive-no-bg)'
                        }}
                    >
                        <Trash2 size={20} />
                    </div>
                    <div class="home-quick-card-info">
                        <span class="home-quick-card-value">
                            {formatCompactNumber(metadata.stats.trash_assets)}
                        </span>
                        <span class="home-quick-card-label">Trash</span>
                        <span class="home-quick-card-subtext">Deleted items</span>
                    </div>
                </button>
            </div>
        </section>
    );
};
