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
import { formatCompactNumber, formatFileSize } from '../../../utils/format';
import { HomeMediaBreakdownProperties, MediaFamilyDisplayConfiguration } from './types';

/**
 * Ordered list of standard media families that must always be displayed.
 */
const STANDARD_MEDIA_FAMILY_KEYS = [
    'image',
    'video',
    'audio',
    'model3d',
    'document',
    'project',
    'font',
    'archive'
] as const;

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
            icon: () => <ImageIcon size={18} />
        },
        video: {
            familyKey: 'video',
            displayLabel: 'Videos & Clips',
            accentColor: 'oklch(65% 0.15 240)',
            accentBackground: 'oklch(65% 0.15 240 / 0.15)',
            icon: () => <VideoIcon size={18} />
        },
        audio: {
            familyKey: 'audio',
            displayLabel: 'Audio & Music',
            accentColor: 'oklch(68% 0.18 310)',
            accentBackground: 'oklch(68% 0.18 310 / 0.15)',
            icon: () => <Music size={18} />
        },
        model3d: {
            familyKey: 'model3d',
            displayLabel: '3D Models',
            accentColor: 'oklch(75% 0.15 85)',
            accentBackground: 'oklch(75% 0.15 85 / 0.15)',
            icon: () => <Box size={18} />
        },
        document: {
            familyKey: 'document',
            displayLabel: 'Documents & Text',
            accentColor: 'oklch(68% 0.14 200)',
            accentBackground: 'oklch(68% 0.14 200 / 0.15)',
            icon: () => <FileText size={18} />
        },
        project: {
            familyKey: 'project',
            displayLabel: 'Creative Projects',
            accentColor: 'oklch(65% 0.18 25)',
            accentBackground: 'oklch(65% 0.18 25 / 0.15)',
            icon: () => <Palette size={18} />
        },
        font: {
            familyKey: 'font',
            displayLabel: 'Typography & Fonts',
            accentColor: 'oklch(70% 0.16 350)',
            accentBackground: 'oklch(70% 0.16 350 / 0.15)',
            icon: () => <Type size={18} />
        },
        archive: {
            familyKey: 'archive',
            displayLabel: 'Archives & Packages',
            accentColor: 'oklch(65% 0.10 50)',
            accentBackground: 'oklch(65% 0.10 50 / 0.15)',
            icon: () => <FolderArchive size={18} />
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
        icon: () => <Layers size={18} />
    };
}

/**
 * Component presenting the breakdown of library assets by media family as a structured table/list.
 * Guarantees that all standard media types are listed, even when having zero assets.
 *
 * @param {HomeMediaBreakdownProperties} properties - Component properties.
 * @returns {JSX.Element} The rendered media breakdown section.
 */
export const HomeMediaBreakdown: Component<HomeMediaBreakdownProperties> = properties => {
    const familyConfigurations = createMediaFamilyConfigurations();

    const normalizedStatistics = createMemo(() => {
        const rawStatistics = properties.familyStatistics;
        const totalSize = properties.totalSizeBytes > 0 ? properties.totalSizeBytes : 1;

        const statisticsMap = new Map<string, { count: number; totalSizeBytes: number }>();
        for (const statisticItem of rawStatistics) {
            statisticsMap.set(statisticItem.family.toLowerCase(), {
                count: statisticItem.count,
                totalSizeBytes: statisticItem.total_size_bytes
            });
        }

        const mergedFamilyKeys = new Set<string>(STANDARD_MEDIA_FAMILY_KEYS);
        for (const rawItem of rawStatistics) {
            mergedFamilyKeys.add(rawItem.family.toLowerCase());
        }

        return Array.from(mergedFamilyKeys).map(familyKey => {
            const statisticData = statisticsMap.get(familyKey) || {
                count: 0,
                totalSizeBytes: 0
            };
            const configuration = resolveFamilyConfiguration(familyKey, familyConfigurations);
            const percentageValue =
                properties.totalSizeBytes > 0
                    ? (statisticData.totalSizeBytes / totalSize) * 100
                    : 0;

            let percentageFormatted = '0%';
            if (percentageValue > 0) {
                percentageFormatted =
                    percentageValue < 0.1 ? '< 0.1%' : `${percentageValue.toFixed(1)}%`;
            }

            return {
                family: familyKey,
                count: statisticData.count,
                totalSizeBytes: statisticData.totalSizeBytes,
                percentage: percentageValue,
                percentageFormatted,
                configuration
            };
        });
    });

    return (
        <section class="home-media-breakdown" aria-label="Media Family Composition">
            <div class="home-section-header">
                <div class="home-section-title-group">
                    <h2>Media Types & Storage</h2>
                    <p>Composition by format and disk footprint</p>
                </div>
            </div>

            {/* Visual Storage Distribution Bar */}
            <div
                class="home-distribution-bar"
                title="Storage distribution bar across media families"
                role="progressbar"
                aria-label="Storage distribution"
            >
                <For each={normalizedStatistics().filter(item => item.totalSizeBytes > 0)}>
                    {statisticItem => (
                        <div
                            class="home-distribution-segment"
                            style={{
                                width: `${Math.max(statisticItem.percentage, 1)}%`,
                                'background-color': statisticItem.configuration.accentColor
                            }}
                            title={`${statisticItem.configuration.displayLabel}: ${formatFileSize(
                                statisticItem.totalSizeBytes
                            )} (${statisticItem.percentageFormatted})`}
                        />
                    )}
                </For>
            </div>

            {/* Media Family List / Table */}
            <div class="home-media-table" role="table" aria-label="Media types table">
                <For each={normalizedStatistics()}>
                    {statisticItem => (
                        <div class="home-media-table-row" role="row">
                            <div class="home-media-row-identity">
                                <div
                                    class="home-media-row-icon"
                                    style={{
                                        background: statisticItem.configuration.accentBackground,
                                        color: statisticItem.configuration.accentColor
                                    }}
                                >
                                    {statisticItem.configuration.icon()}
                                </div>
                                <span class="home-media-row-name">
                                    {statisticItem.configuration.displayLabel}
                                </span>
                            </div>

                            <div class="home-media-row-metrics">
                                <span class="home-media-row-count">
                                    {formatCompactNumber(statisticItem.count)}
                                </span>
                                <span class="home-media-row-size">
                                    {formatFileSize(statisticItem.totalSizeBytes)}
                                </span>
                            </div>

                            <div class="home-media-row-bar-wrapper">
                                <div class="home-media-row-bar-track">
                                    <div
                                        class="home-media-row-bar-fill"
                                        style={{
                                            width: `${Math.max(statisticItem.percentage, 0)}%`,
                                            background: statisticItem.configuration.accentColor
                                        }}
                                    />
                                </div>
                                <span class="home-media-row-percentage">
                                    {statisticItem.percentageFormatted}
                                </span>
                            </div>
                        </div>
                    )}
                </For>
            </div>
        </section>
    );
};
