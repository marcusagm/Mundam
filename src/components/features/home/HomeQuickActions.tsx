import { Component } from 'solid-js';
import { GalleryVerticalEnd, FileStack, Tag, Trash2 } from 'lucide-solid';
import { filterActions } from '../../../core/store/filter';
import { HomeQuickActionsProperties } from './types';

/**
 * Toolbar presenting direct shortcuts to high-frequency workflows.
 * Enables quick triage (duplicates, untagged assets) and instant gallery navigation.
 *
 * @param {HomeQuickActionsProperties} properties - Component properties.
 * @returns {JSX.Element} The rendered quick actions toolbar.
 */
export const HomeQuickActions: Component<HomeQuickActionsProperties> = properties => {
    /**
     * Handles navigation to the general Gallery view.
     *
     * @returns {void}
     */
    const handleNavigateToGallery = (): void => {
        filterActions.clearAll();
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
     * Handles navigation to Untagged assets for rapid tagging.
     *
     * @returns {void}
     */
    const handleNavigateToUntagged = (): void => {
        filterActions.clearAll();
        filterActions.setUntagged(true);
        properties.onNavigate('gallery');
    };

    /**
     * Handles navigation to the Trash folder.
     *
     * @returns {void}
     */
    const handleNavigateToTrash = (): void => {
        filterActions.clearAll();
        filterActions.setTrash(true);
        properties.onNavigate('gallery');
    };

    return (
        <section class="home-quick-actions" aria-label="Quick Navigation Actions">
            <div class="home-section-header">
                <div class="home-section-title-group">
                    <h2>Quick Management Hub</h2>
                    <p>Direct shortcuts for common library maintenance and asset discovery tasks</p>
                </div>
            </div>

            <div class="home-quick-actions-grid">
                <button
                    type="button"
                    class="home-quick-action-button"
                    onClick={handleNavigateToGallery}
                >
                    <div
                        class="home-stat-icon-wrapper"
                        style={{
                            background: 'oklch(58% var(--theme-c) var(--theme-h) / 0.15)',
                            color: 'var(--action-primary-bg)'
                        }}
                    >
                        <GalleryVerticalEnd size={22} />
                    </div>
                    <div>
                        <h3 class="home-quick-action-title">Browse Entire Gallery</h3>
                        <p class="home-quick-action-description">
                            Explore and inspect all assets in the visual grid
                        </p>
                    </div>
                </button>

                <button
                    type="button"
                    class="home-quick-action-button"
                    onClick={handleNavigateToDuplicates}
                >
                    <div
                        class="home-stat-icon-wrapper"
                        style={{
                            background: 'oklch(75% 0.15 85 / 0.15)',
                            color: 'oklch(80% 0.13 85)'
                        }}
                    >
                        <FileStack size={22} />
                    </div>
                    <div>
                        <h3 class="home-quick-action-title">Find Duplicates</h3>
                        <p class="home-quick-action-description">
                            Scan and resolve identical and perceptual copies
                        </p>
                    </div>
                </button>

                <button
                    type="button"
                    class="home-quick-action-button"
                    onClick={handleNavigateToUntagged}
                >
                    <div
                        class="home-stat-icon-wrapper"
                        style={{
                            background: 'oklch(65% 0.18 145 / 0.15)',
                            color: 'oklch(70% 0.15 145)'
                        }}
                    >
                        <Tag size={22} />
                    </div>
                    <div>
                        <h3 class="home-quick-action-title">Organize Untagged</h3>
                        <p class="home-quick-action-description">
                            Batch tag unclassified assets for better searchability
                        </p>
                    </div>
                </button>

                <button
                    type="button"
                    class="home-quick-action-button"
                    onClick={handleNavigateToTrash}
                >
                    <div
                        class="home-stat-icon-wrapper"
                        style={{
                            background: 'oklch(60% 0.20 25 / 0.15)',
                            color: 'var(--text-destructive-no-bg)'
                        }}
                    >
                        <Trash2 size={22} />
                    </div>
                    <div>
                        <h3 class="home-quick-action-title">Manage Trash</h3>
                        <p class="home-quick-action-description">
                            Review and purge or restore deleted references
                        </p>
                    </div>
                </button>
            </div>
        </section>
    );
};
