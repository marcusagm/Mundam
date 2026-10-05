/**
 * VirtualList Component Module
 *
 * @module VirtualList
 * @description
 * High-performance virtualized list and container for Solid.js.
 * Implements windowed rendering for large collections by rendering only the items
 * currently intersecting the visible viewport plus configurable buffer overscan.
 *
 * Key features:
 * - Constant-time arithmetic layout for uniform fixed item heights.
 * - Dynamic height resolution with binary-search lookups for variable item heights.
 * - Configurable item spacing (gap) and overscan buffers.
 * - Animation-frame throttled scroll tracking via the core scheduler.
 * - Debounced visible item change notifications for efficient lazy-loading and preloading.
 * - Programmatic controller for smooth scrolling and keyboard selection navigation.
 * - Full accessibility compliance with ARIA roles and position attributes.
 *
 * @example
 * ```tsx
 * import { VirtualList } from '@/components/ui';
 *
 * <VirtualList
 *     items={groups()}
 *     itemHeight={150}
 *     gap={8}
 *     overscanCount={5}
 *     onVisibleItemsChange={(visibleGroups) => preloadThumbnails(visibleGroups)}
 * >
 *     {(groupItem, indexAccessor) => (
 *         <GroupCard item={groupItem} index={indexAccessor()} />
 *     )}
 * </VirtualList>
 * ```
 */

export * from './types';
export * from './VirtualList';
export * from './hooks/useVirtualList';
