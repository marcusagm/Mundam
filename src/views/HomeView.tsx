import { Component, JSX, onMount } from 'solid-js';
import { ApplicationView } from '../components/layout/TitleBar/types';
import { useMetadata } from '../core/hooks';
import { HomeQuickAccess, HomeMediaBreakdown, HomeRecentStream } from '../components/features/home';
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
 * - Quick access counters ribbon for all key library dimensions
 * - Media family and disk footprint composition breakdown (table/list)
 * - Stream of most recently added or indexed assets (occupying 2/3 width)
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
                <HomeQuickAccess onNavigate={handleNavigate} />
                <div class="home-columns-container">
                    <HomeMediaBreakdown
                        familyStatistics={metadata.stats.family_stats}
                        totalSizeBytes={metadata.stats.total_size_bytes}
                        onNavigate={handleNavigate}
                    />
                    <HomeRecentStream onNavigate={handleNavigate} />
                </div>
            </main>
        </div>
    );
};
