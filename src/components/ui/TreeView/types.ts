import { JSX, Component } from 'solid-js';
import { DragItem } from '../../../core/dnd';

/**
 * Valid relative positions for a drop operation within the tree.
 */
export type TreeDropPosition = 'before' | 'inside' | 'after';

/**
 * Generic tree node structure.
 * @template T - Type of the additional data stored in the node.
 */
export interface TreeNode<T = unknown> {
    /**
     * Unique identifier for the node.
     * Must be unique within the entire tree.
     */
    id: string | number;

    /**
     * Human-readable text displayed for the node.
     */
    label: string;

    /**
     * Optional icon component to display for the node.
     */
    icon?: Component;

    /**
     * Optional specific color for the icon.
     * This is used to override the default icon color.
     */
    iconColor?: string;
    /** Nested child nodes */
    children?: TreeNode<T>[];
    /** Optional secondary element (badge, count, etc.) to display next to the label */
    badge?: JSX.Element;
    /** Additional business logic data */
    data?: T;
}

/**
 * Shared properties for the TreeView system.
 *
 * This interface defines the contract for initializing and controlling the TreeView components.
 * It follows a generic pattern to allow nodes to hold arbitrary business data.
 *
 * @template T - The type of business data stored within each tree node.
 */
export interface TreeViewProps<T = unknown> {
    /** The hierarchical list of nodes to be displayed in the tree. */
    items: TreeNode<T>[];
    /** Optional CSS class to be applied to the root tree container. */
    class?: string;
    /** The indentation distance in pixels for each nesting level. Defaults to 16. */
    indentSize?: number;
    /** A set of node IDs that are currently expanded in the UI. */
    expandedIds?: Set<string | number>;
    /** An array of node IDs that are currently visually selected. */
    selectedIds?: (string | number)[];
    /** The unique identifier of the node currently undergoing a rename/edit operation. */
    editingId?: string | number | null;
    /** The fallback icon component to use if a node does not provide its own specific icon. */
    defaultIcon?: Component;
    /** Whether nodes within the tree are interactive and can be dragged. */
    draggable?: boolean;
    /** The specific drag-and-drop type identifier used for the nodes of this tree (e.g., 'TAG'). */
    dragType?: string;
    /** The list of drag-and-drop type identifiers that this tree considers valid targets for dropping. */
    acceptedDragTypes?: string[];

    /** Callback function invoked when a node is clicked or selected. */
    /** `isAdditive` is true for Ctrl/Meta+click. */
    onSelect?: (node: TreeNode<T>, isAdditive?: boolean) => void;
    /** Callback function invoked when a node expansion state is toggled. */
    onToggle?: (id: string | number) => void;
    /** Callback function invoked when a context menu is requested for a specific node. */
    onContextMenu?: (event: MouseEvent, node: TreeNode<T>) => void;
    /** Callback function invoked when a rename/edit operation is successfully committed. */
    onRename?: (node: TreeNode<T>, newLabel: string) => void;
    /** Callback function invoked when rename/edit mode is exited without saving. */
    onEditCancel?: () => void;
    /** Callback function invoked when an item is dropped onto a target in the tree. */
    onDrop?: (
        item: DragItem,
        targetId: string | number | 'root',
        position: TreeDropPosition
    ) => void;
    /** Optional custom validation function to determine if a specific drop operation is valid. */
    isValidDrop?: (dragged: DragItem, target: TreeNode<T>) => boolean;
}

/**
 * Internal properties used by the TreeViewItem component.
 * Extends the base TreeViewProps with depth tracking and unique DOM context.
 *
 * @template T - The type of business data in nodes.
 */
export interface TreeViewItemProps<T = unknown> extends TreeViewProps<T> {
    /** The specific node instance to render in this item. */
    node: TreeNode<T>;
    /** The nesting depth of the node (0 for root level). */
    depth: number;
    /** The unique identifier of the parent tree container for DOM association. */
    treeId: string;
    /** Whether this node is the last sibling in its parent's group (used for rendering guide lines). */
    isLast: boolean;
}
