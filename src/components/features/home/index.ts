/**
 * Home Feature Module
 *
 * @module Home
 * @description
 * High-level dashboard and asset command center providing actionable insights,
 * storage distribution breakdowns, taxonomy health indicators, and a recent ingestion stream.
 *
 * @example
 * ```tsx
 * import { HomeHeroMetrics, HomeMediaBreakdown, HomeRecentStream } from '@/components/features/home';
 *
 * <HomeHeroMetrics onNavigate={handleNavigate} />
 * <HomeMediaBreakdown
 *     familyStatistics={stats.family_stats}
 *     totalSizeBytes={stats.total_size_bytes}
 *     onNavigate={handleNavigate}
 * />
 * <HomeRecentStream onNavigate={handleNavigate} />
 * ```
 */

export * from './types';
export * from './HomeQuickAccess';
export * from './HomeMediaBreakdown';
export * from './HomeRecentStream';
