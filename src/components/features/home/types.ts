import { JSX } from 'solid-js';
import { ApplicationView } from '../../layout/TitleBar/types';
import { FamilyStat } from '../../../lib/tags';
import { AssetItem } from '../../../types';

/**
 * Visual configuration for a media family category.
 */
export interface MediaFamilyDisplayConfiguration {
    /** The machine family key returned by backend (e.g., 'image', 'video', 'audio') */
    familyKey: string;
    /** Human-readable localized title */
    displayLabel: string;
    /** Accent color variable or OKLCH value */
    accentColor: string;
    /** Background accent color */
    accentBackground: string;
    /** Icon rendering function */
    icon: () => JSX.Element;
}

/**
 * Properties for the quick access counters grid component.
 */
export interface HomeQuickAccessProperties {
    /** Callback to switch application view */
    onNavigate: (view: ApplicationView) => void;
}

/**
 * Properties for the media family breakdown component.
 */
export interface HomeMediaBreakdownProperties {
    /** List of family statistics from the library state */
    familyStatistics: FamilyStat[];
    /** Total library size in bytes */
    totalSizeBytes: number;
}

/**
 * Properties for the recent assets stream component.
 */
export interface HomeRecentStreamProperties {
    /** Callback to switch application view */
    onNavigate: (view: ApplicationView) => void;
}

/**
 * Properties for an individual recent asset card component.
 */
export interface RecentAssetCardProperties {
    /** The asset data */
    assetItem: AssetItem;
    /** Click handler */
    onSelectAsset: (assetItem: AssetItem) => void;
}
