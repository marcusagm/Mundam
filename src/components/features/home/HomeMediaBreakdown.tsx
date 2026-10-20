import { Component, For, createMemo } from 'solid-js';
import {
    Image as ImageIcon,
    Video as VideoIcon,
    Music,
    Box,
    FileText,
    Palette,
    Type,
    FolderArchive,
    Layers
} from 'lucide-solid';
import { filterActions } from '../../../core/store/filter';
import { formatCompactNumber, formatFileSize } from '../../../utils/format';
import { HomeMediaBreakdownProperties, MediaFamilyDisplayConfiguration } from './types';

/**
 * Builds the display configuration mapping for each media family.
 *
 * @returns {Record<string, MediaFamilyDisplayConfiguration>} The configuration mapping.
 */
function createMediaFamilyConfigurations(): Record<string, MediaFamilyDisplayConfiguration> {
    return {
        image: {
            familyKey: 'image',
            displayLabel: 'Images & Photos',
            accentColor: 'oklch(65% 0.18 145)',
            accentBackground: 'oklch(65% 0.18 145 / 0.15)',
            icon: () => <ImageIcon size={20} />
        },
        video: {
            familyKey: 'video',
            displayLabel: 'Videos & Clips',
            accentColor: 'oklch(65% 0.15 240)',
            accentBackground: 'oklch(65% 0.15 240 / 0.15)',
            icon: () => <VideoIcon size={20} />
        },
        audio: {
            familyKey: 'audio',
            displayLabel: 'Audio & Music',
            accentColor: 'oklch(68% 0.18 310)',
            accentBackground: 'oklch(68% 0.18 310 / 0.15)',
            icon: () => <Music size={20} />
        },
        model3d: {
            familyKey: 'model3d',
            displayLabel: '3D Models & Assets',
            accentColor: 'oklch(75% 0.15 85)',
            accentBackground: 'oklch(75% 0.15 85 / 0.15)',
            icon: () => <Box size={20} />
        },
        document: {
            familyKey: 'document',
            displayLabel: 'Documents & Texts',
            accentColor: 'oklch(68% 0.14 200)',
            accentBackground: 'oklch(68% 0.14 200 / 0.15)',
            icon: () => <FileText size={20} />
        },
        project: {
            familyKey: 'project',
            displayLabel: 'Creative Projects',
            accentColor: 'oklch(65% 0.18 25)',
            accentBackground: 'oklch(65% 0.18 25 / 0.15)',
            icon: () => <Palette size={20} />
        },
        font: {
            familyKey: 'font',
            displayLabel: 'Fonts & Typography',
            accentColor: 'oklch(70% 0.16 350)',
            accentBackground: 'oklch(70% 0.16 350 / 0.15)',
            icon: () => <Type size={20} />
        },
        archive: {
            familyKey: 'archive',
            displayLabel: 'Archives & Packages',
            accentColor: 'oklch(65% 0.10 50)',
            accentBackground: 'oklch(65% 0.10 50 / 0.15)',
            icon: () => <FolderArchive size={20} />
        }
    };
}

/**
 * Resolves the configuration for a given media family key.
 *
 * @param {string} familyKey - The media family key.
 * @param {Record<string, MediaFamilyDisplayConfiguration>} configurations - Available configurations.
 * @returns {MediaFamilyDisplayConfiguration} The resolved configuration.
 */
function resolveFamilyConfiguration(
    familyKey: string,
    configurations: Record<string, MediaFamilyDisplayConfiguration>
): MediaFamilyDisplayConfiguration {
    const normalizedKey = familyKey.toLowerCase();
    if (configurations[normalizedKey]) {
        return configurations[normalizedKey];
    }
    return {
        familyKey: normalizedKey,
        displayLabel: normalizedKey.charAt(0).toUpperCase() + normalizedKey.slice(1),
        accentColor: 'oklch(60% 0.05 240)',
        accentBackground: 'oklch(60% 0.05 240 / 0.15)',
        icon: () => <Layers size={20} />
    };
}

/**
 * Component presenting the breakdown of library assets by media family.
 * Renders a segmented distribution bar and individual cards with file count, disk space, and filter deep-linking.
 *
 * @param {HomeMediaBreakdownProperties} properties - Component properties.
 * @returns {JSX.Element} The rendered media breakdown section.
 */
export const HomeMediaBreakdown: Component<HomeMediaBreakdownProperties> = properties => {
    const familyConfigurations = createMediaFamilyConfigurations();

    const normalizedStatistics = createMemo(() => {
        const rawStatistics = properties.familyStatistics;
        const totalSize = properties.totalSizeBytes > 0 ? properties.totalSizeBytes : 1;

        return rawStatistics.map(statisticItem => {
            const configuration = resolveFamilyConfiguration(
                statisticItem.family,
                familyConfigurations
            );
            const percentageValue = (statisticItem.total_size_bytes / totalSize) * 100;

            return {
                family: statisticItem.family,
                count: statisticItem.count,
                totalSizeBytes: statisticItem.total_size_bytes,
                percentage: percentageValue,
                percentageFormatted:
                    percentageValue < 0.1 && percentageValue > 0
                        ? '< 0.1%'
                        : `${percentageValue.toFixed(1)}%`,
                configuration
            };
        });
    });

    /**
     * Handles selecting a media family, filtering the gallery, and navigating.
     *
     * @param {string} familyKey - The selected media family key.
     * @returns {void}
     */
    const handleFamilyCardClick = (familyKey: string): void => {
        filterActions.clearAll();
        filterActions.setFamily(familyKey);
        properties.onNavigate('gallery');
    };

    return (
        <section class="home-media-breakdown" aria-label="Media Family Composition">
            <div class="home-section-header">
                <div class="home-section-title-group">
                    <h2>Library Composition & Storage</h2>
                    <p>Disk distribution and file numbers across all media formats</p>
                </div>
            </div>

            {/* Visual Storage Distribution Bar */}
            <div
                class="home-distribution-bar"
                title="Storage distribution bar across media families"
                role="progressbar"
                aria-label="Storage distribution"
            >
                <For each={normalizedStatistics()}>
                    {statisticItem => (
                        <div
                            class="home-distribution-segment"
                            style={{
                                width: `${Math.max(statisticItem.percentage, 0.5)}%`,
                                'background-color': statisticItem.configuration.accentColor
                            }}
                            title={`${statisticItem.configuration.displayLabel}: ${formatFileSize(
                                statisticItem.totalSizeBytes
                            )} (${statisticItem.percentageFormatted})`}
                        />
                    )}
                </For>
            </div>

            {/* Media Family Cards Grid */}
            <div class="home-media-grid">
                <For each={normalizedStatistics()}>
                    {statisticItem => (
                        <button
                            type="button"
                            class="home-media-card"
                            onClick={() => handleFamilyCardClick(statisticItem.family)}
                            title={`Browse ${statisticItem.configuration.displayLabel} in Gallery`}
                        >
                            <div class="home-media-card-top">
                                <div
                                    class="home-media-card-icon"
                                    style={{
                                        background: statisticItem.configuration.accentBackground,
                                        color: statisticItem.configuration.accentColor
                                    }}
                                >
                                    {statisticItem.configuration.icon()}
                                </div>
                                <span class="home-media-card-percentage">
                                    {statisticItem.percentageFormatted}
                                </span>
                            </div>

                            <h3 class="home-media-card-title">
                                {statisticItem.configuration.displayLabel}
                            </h3>

                            <div class="home-media-card-progress">
                                <div
                                    class="home-media-card-progress-fill"
                                    style={{
                                        width: `${Math.max(statisticItem.percentage, 2)}%`,
                                        background: statisticItem.configuration.accentColor
                                    }}
                                />
                            </div>

                            <div class="home-media-card-meta">
                                <span>{formatCompactNumber(statisticItem.count)} items</span>
                                <span class="home-media-card-size">
                                    {formatFileSize(statisticItem.totalSizeBytes)}
                                </span>
                            </div>
                        </button>
                    )}
                </For>
            </div>
        </section>
    );
};
