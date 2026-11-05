/**
 * Context Menu component
 *
 * @module ContextMenu
 * @description
 * The ContextMenu component is a coordinate-based menu triggered by right-clicks.
 * It provides a menu that appears at the mouse position when the user right-clicks on the trigger element.
 * Supports submenus and keyboard navigation.
 *
 * @example
 * ```tsx
 * import { ContextMenu } from '@/components/ui';
 *
 * <ContextMenu
 *   coordinateX={0}
 *   coordinateY={0}
 *   items={[
 *     {
 *       label: 'Item 1',
 *       onClick: () => {},
 *     },
 *     {
 *       label: 'Item 2',
 *       onClick: () => {},
 *     },
 *   ]}
 *   isOpen={true}
 *   onClose={() => {}}
 * />
 * ```
 */

export { ContextMenu } from './ContextMenu';
export type { ContextMenuProps, ContextMenuItem } from './types';
export * from './types';

export { ContextMenuCheckboxItem } from './ContextMenuCheckboxItem';
