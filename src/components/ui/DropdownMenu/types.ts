import { Component, JSX, Accessor } from 'solid-js';

/**
 * Base properties shared by most dropdown menu items.
 */
interface BaseMenuItem {
    /** Unique identifier for the item. */
    id?: string;
    /** Display text for the item. */
    label: string;
    /** Optional icon component. */
    icon?: Component<{ size?: number; class?: string }>;
    /** Whether the item is disabled. */
    disabled?: boolean;
}

/**
 * A standard clickable menu item.
 */
export interface ActionMenuItem extends BaseMenuItem {
    type: 'item';
    /** Action to execute when the item is clicked or selected. */
    action: () => void;
    /** Keyboard shortcut string for display. */
    shortcut?: string;
}

/**
 * A toggleable checkbox menu item.
 */
export interface CheckboxMenuItem extends BaseMenuItem {
    type: 'checkbox';
    /** Current checked state. */
    checked: boolean;
    /** Callback when the checked state changes. */
    onCheckedChange: (checked: boolean) => void;
}

/**
 * A radio menu item for selecting one value from a group.
 */
export interface RadioMenuItem extends BaseMenuItem {
    type: 'radio';
    /** Value associated with this radio item. */
    value: string;
}

/**
 * A recursive submenu.
 */
export interface SubmenuMenuItem extends BaseMenuItem {
    type: 'submenu';
    /** List of items contained within this submenu. */
    items: DropdownMenuItem[];
}

/**
 * A non-interactive label for grouping items.
 */
export interface LabelMenuItem {
    type: 'label';
    /** Display text for the label. */
    label: string;
}

/**
 * A visual separator between items or groups.
 */
export interface SeparatorMenuItem {
    type: 'separator';
}

/**
 * Discriminated union of all possible dropdown menu item types.
 * This ensures type safety and eliminates the need for 'as unknown' casts.
 */
export type DropdownMenuItem =
    | ActionMenuItem
    | CheckboxMenuItem
    | RadioMenuItem
    | SubmenuMenuItem
    | LabelMenuItem
    | SeparatorMenuItem;

/**
 * Alignment options for the dropdown content relative to the trigger.
 */
export type DropdownAlignment = 'start' | 'center' | 'end';

/**
 * Side options for where the dropdown appears relative to the trigger.
 */
export type DropdownSide = 'top' | 'bottom' | 'left' | 'right';

/**
 * Properties for the main DropdownMenu component.
 */
export interface DropdownMenuProps {
    /** Trigger element or a renderer that receives the current expanded state. */
    trigger: JSX.Element | ((expanded: Accessor<boolean>) => JSX.Element);
    /** Array of menu items to display. */
    items: DropdownMenuItem[];
    /** Preferred alignment of the menu. Defaults to 'start'. */
    align?: DropdownAlignment;
    /** Preferred side of the menu. Defaults to 'bottom'. */
    side?: DropdownSide;
    /** Currently selected value for radio items in this menu. */
    radioValue?: string;
    /** Callback when a radio value changes. */
    onRadioChange?: (value: string) => void;
    /** Optional CSS class for the root wrapper. */
    class?: string;
    /** Optional CSS class for the menu content. */
    contentClass?: string;
}

/**
 * Internal context for the dropdown to share state between root and items.
 */
export interface DropdownContextValue {
    /** Closes the entire menu tree. */
    close: () => void;
    /** Accessor for the active radio value. */
    radioValue?: Accessor<string>;
    /** Handler for radio item selection. */
    onRadioChange?: (value: string) => void;
}
