import { Component, JSX } from 'solid-js';

/**
 * Common properties for all context menu items.
 */
interface BaseContextMenuItem {
    /** Unique display label for the item. */
    label: string;
    /** Optional icon component. */
    icon?: Component<{ size?: number; class?: string }>;
    /** Whether the item is disabled. */
    disabled?: boolean;
}

/**
 * Standard selectable context menu item.
 */
export interface ActionContextMenuItem extends BaseContextMenuItem {
    type: 'item';
    /** Action to perform when selected. */
    action: () => void;
    /** Keyboard shortcut for display. */
    shortcut?: string;
    /** Visual hint that the action is dangerous (e.g., delete). */
    danger?: boolean;
}

/**
 * Recursive submenu.
 */
export interface SubmenuContextMenuItem extends BaseContextMenuItem {
    type: 'submenu';
    /** Nested items for this submenu. */
    items: ContextMenuItem[];
}

/**
 * Custom content item for advanced context menu layouts.
 */
export interface CustomContextMenuItem {
    type: 'custom';
    /** Content to be rendered inside the item. */
    content: JSX.Element;
}

/**
 * Visual separator for grouping items.
 */
export interface SeparatorContextMenuItem {
    type: 'separator';
}

/**
 * Discriminated union of all possible context menu items.
 * Ensures type safety and eliminates the need for 'as unknown' casts.
 */
export type ContextMenuItem =
    | ActionContextMenuItem
    | SubmenuContextMenuItem
    | CustomContextMenuItem
    | SeparatorContextMenuItem;

/**
 * Properties for the ContextMenu component.
 */
export interface ContextMenuProps {
    /** X coordinate for positioning. */
    coordinateX: number;
    /** Y coordinate for positioning. */
    coordinateY: number;
    /** Menu items to display. */
    items: ContextMenuItem[];
    /** Whether the menu is currently visible. */
    isOpen: boolean;
    /** Callback to request menu closure. */
    onClose: () => void;
}

/** Accessible preference row rendered inside a custom context-menu item. */
export interface ContextMenuCheckboxItemProps {
    /** Visible preference label. */
    label: string;
    /** Current reactive checked state. */
    checked: boolean;
    /** Commits the new preference before closing. */
    onCheckedChange: (checked: boolean) => void;
    /** Closes the enclosing menu after activation. */
    onClose: () => void;
}
