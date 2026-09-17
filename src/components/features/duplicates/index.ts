/**
 * Duplicates Feature Module
 *
 * @module Duplicates
 * @description
 * Public surface for the duplicate detection UI feature. Exposes the domain types,
 * list and comparison panel components, and the split-view comparison modal.
 *
 * The hook `useDuplicateGroups` is NOT exported here — it is consumed directly
 * by `DuplicateFinderView` to keep the data-fetching boundary at the view layer.
 *
 * @example
 * ```tsx
 * import { DuplicateGroupList, DuplicateComparisonPanel } from '@/components/features/duplicates';
 * ```
 */
export * from './types';
export * from './DuplicateGroupList';
export * from './DuplicateGroupItem';
export * from './DuplicateComparisonPanel';
export * from './DuplicateSplitView';
export * from './DuplicateCandidateCard';
