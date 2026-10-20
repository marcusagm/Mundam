import { Component, JSX, onMount } from 'solid-js';
import { ApplicationView } from '../components/layout/TitleBar/types';
import { useMetadata } from '../core/hooks';
import {
    HomeHeroMetrics,
    HomeMediaBreakdown,
    HomeRecentStream,
    HomeQuickActions
} from '../components/features/home';
import '../components/features/home/home.css';

export interface HomeViewProperties {
    /** The title bar to render at the top */
    header: JSX.Element;
    /** Callback to change active application view */
    onNavigate?: (targetView: ApplicationView) => void;
}

/**
 * Main Home view of the application.
 * Functions as the central Asset Command Center with:
 * - Library pulse metrics and quick filter teleport chips
 * - Media family and disk footprint composition breakdown
 * - Stream of most recently added or indexed assets
 * - Quick management and discovery shortcuts
 *
 * @param {HomeViewProperties} properties - Component properties.
 * @returns {JSX.Element} The rendered home view.
 *
 * @example
 * ```tsx
 * import { HomeView } from '@/views';
 * <HomeView header={<TitleBar />} onNavigate={handleNavigate} />
 * ```
 */
export const HomeView: Component<HomeViewProperties> = properties => {
    const metadata = useMetadata();

    onMount(() => {
        metadata.loadStats();
    });

    /**
     * Handles view navigation requested by child components.
     *
     * @param {ApplicationView} targetView - The target application view.
     * @returns {void}
     */
    const handleNavigate = (targetView: ApplicationView): void => {
        if (properties.onNavigate) {
            properties.onNavigate(targetView);
        }
    };

    return (
        <div class="home-container">
            {properties.header}
            <main class="home-scroll-area">
                <HomeHeroMetrics onNavigate={handleNavigate} />
                <HomeQuickActions onNavigate={handleNavigate} />
                <HomeMediaBreakdown
                    familyStatistics={metadata.stats.family_stats}
                    totalSizeBytes={metadata.stats.total_size_bytes}
                    onNavigate={handleNavigate}
                />
                <HomeRecentStream onNavigate={handleNavigate} />
            </main>
        </div>
    );
};
