import { Component, createMemo } from 'solid-js';
import {
    HardDrive,
    FolderTree,
    Tags,
    Star,
    Trash2,
    SlidersHorizontal,
    Copy,
    Tag
} from 'lucide-solid';
import { useMetadata } from '../../../core/hooks';
import { filterActions } from '../../../core/store/filter';
import { formatCompactNumber, formatFileSize } from '../../../utils/format';
import { HomeHeroMetricsProperties } from './types';

/**
 * Top metrics ribbon for the Home view.
 * Displays primary library statistics (total assets, storage footprint, watched locations, taxonomy health)
 * and actionable teleport chips that deep-link directly into the Gallery or Duplicates view.
 *
 * @param {HomeHeroMetricsProperties} properties - Component properties.
 * @returns {JSX.Element} The rendered hero metrics ribbon.
 */
export const HomeHeroMetrics: Component<HomeHeroMetricsProperties> = properties => {
    const metadata = useMetadata();

    const taggedAssetsPercentage = createMemo(() => {
        const totalAssetsCount = metadata.stats.total_assets;
        if (totalAssetsCount === 0) return 0;
        const hasTagsCount = metadata.stats.has_tags_assets;
        return Math.round((hasTagsCount / totalAssetsCount) * 100);
    });

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
     * Handles navigation to the Gallery with the Has Tags filter activated.
     *
     * @returns {void}
     */
    const handleNavigateToHasTags = (): void => {
        filterActions.clearAll();
        filterActions.setHasTags(true);
        properties.onNavigate('gallery');
    };

    /**
     * Handles navigation to the Duplicates view.
     *
     * @returns {void}
     */
    const handleNavigateToDuplicates = (): void => {
        properties.onNavigate('duplicates');
    };

    /**
     * Handles navigation to the general Gallery view.
     *
     * @returns {void}
     */
    const handleNavigateToGallery = (): void => {
        filterActions.clearAll();
        properties.onNavigate('gallery');
    };

    return (
        <section class="home-hero" aria-label="Library Overview Metrics">
            <div class="home-hero-header">
                <div class="home-hero-title-group">
                    <h1>Asset Command Center</h1>
                    <p>Comprehensive overview and health of your multimedia library</p>
                </div>

                <div class="home-taxonomy-bar-container" title="Taxonomy organization ratio">
                    <div class="home-taxonomy-labels">
                        <span>Organization Health</span>
                        <span>{taggedAssetsPercentage()}% Tagged</span>
                    </div>
                    <div class="home-taxonomy-track">
                        <div
                            class="home-taxonomy-fill"
                            style={{ width: `${taggedAssetsPercentage()}%` }}
                        />
                    </div>
                </div>
            </div>

            <div class="home-hero-stats-row">
                <div class="home-stat-card">
                    <div
                        class="home-stat-icon-wrapper"
                        style={{
                            background: 'oklch(58% var(--theme-c) var(--theme-h) / 0.15)',
                            color: 'var(--action-primary-bg)'
                        }}
                    >
                        <HardDrive size={22} />
                    </div>
                    <div class="home-stat-content">
                        <span class="home-stat-label">Total Assets</span>
                        <span class="home-stat-value">
                            {formatCompactNumber(metadata.stats.total_assets)}
                        </span>
                        <span class="home-stat-subtext">
                            {formatFileSize(metadata.stats.total_size_bytes)} on disk
                        </span>
                    </div>
                </div>

                <div class="home-stat-card">
                    <div
                        class="home-stat-icon-wrapper"
                        style={{
                            background: 'oklch(65% 0.15 240 / 0.15)',
                            color: 'oklch(71% 0.14 240)'
                        }}
                    >
                        <FolderTree size={22} />
                    </div>
                    <div class="home-stat-content">
                        <span class="home-stat-label">Watched Folders</span>
                        <span class="home-stat-value">
                            {formatCompactNumber(metadata.stats.total_folders)}
                        </span>
                        <span class="home-stat-subtext">Indexed directories</span>
                    </div>
                </div>

                <div class="home-stat-card">
                    <div
                        class="home-stat-icon-wrapper"
                        style={{
                            background: 'oklch(65% 0.18 145 / 0.15)',
                            color: 'oklch(70% 0.15 145)'
                        }}
                    >
                        <Tags size={22} />
                    </div>
                    <div class="home-stat-content">
                        <span class="home-stat-label">Taxonomy Tags</span>
                        <span class="home-stat-value">
                            {formatCompactNumber(metadata.stats.total_tags)}
                        </span>
                        <span class="home-stat-subtext">Classification labels</span>
                    </div>
                </div>

                <div class="home-stat-card">
                    <div
                        class="home-stat-icon-wrapper"
                        style={{
                            background: 'oklch(75% 0.15 85 / 0.15)',
                            color: 'oklch(80% 0.13 85)'
                        }}
                    >
                        <Copy size={22} />
                    </div>
                    <div class="home-stat-content">
                        <span class="home-stat-label">Duplicates</span>
                        <span class="home-stat-value">
                            {formatCompactNumber(metadata.stats.duplicate_assets)}
                        </span>
                        <span class="home-stat-subtext">Detected candidate items</span>
                    </div>
                </div>
            </div>

            <div class="home-teleport-ribbon" aria-label="Quick Filter Shortcuts">
                <button
                    type="button"
                    class="home-teleport-chip"
                    onClick={handleNavigateToFavorites}
                    title="Filter by favorite assets"
                >
                    <Star size={14} style={{ color: 'oklch(80% 0.13 85)' }} />
                    <span>Favorites</span>
                    <span class="home-teleport-badge">
                        {formatCompactNumber(metadata.stats.favorite_assets)}
                    </span>
                </button>

                <button
                    type="button"
                    class="home-teleport-chip"
                    onClick={handleNavigateToHasTags}
                    title="Filter assets with tags"
                >
                    <Tag size={14} style={{ color: 'oklch(70% 0.15 145)' }} />
                    <span>Tagged Assets</span>
                    <span class="home-teleport-badge">
                        {formatCompactNumber(metadata.stats.has_tags_assets)}
                    </span>
                </button>

                <button
                    type="button"
                    class="home-teleport-chip"
                    onClick={handleNavigateToUntagged}
                    title="Filter untagged assets to organize them"
                >
                    <Tags size={14} style={{ color: 'var(--text-secondary)' }} />
                    <span>Untagged Assets</span>
                    <span class="home-teleport-badge">
                        {formatCompactNumber(metadata.stats.untagged_assets)}
                    </span>
                </button>

                <button
                    type="button"
                    class="home-teleport-chip"
                    onClick={handleNavigateToGallery}
                    title="View smart search presets"
                >
                    <SlidersHorizontal size={14} style={{ color: 'oklch(71% 0.14 240)' }} />
                    <span>Smart Folders</span>
                    <span class="home-teleport-badge">
                        {formatCompactNumber(metadata.stats.smart_folders)}
                    </span>
                </button>

                <button
                    type="button"
                    class="home-teleport-chip"
                    classList={{
                        'home-teleport-chip--warning': metadata.stats.duplicate_assets > 0
                    }}
                    onClick={handleNavigateToDuplicates}
                    title="Open Duplicate Finder view"
                >
                    <Copy size={14} />
                    <span>Duplicate Candidates</span>
                    <span class="home-teleport-badge">
                        {formatCompactNumber(metadata.stats.duplicate_assets)}
                    </span>
                </button>

                <button
                    type="button"
                    class="home-teleport-chip"
                    onClick={handleNavigateToTrash}
                    title="View assets currently in trash"
                >
                    <Trash2 size={14} style={{ color: 'var(--text-destructive-no-bg)' }} />
                    <span>Trash</span>
                    <span class="home-teleport-badge">
                        {formatCompactNumber(metadata.stats.trash_assets)}
                    </span>
                </button>
            </div>
        </section>
    );
};
