import { createShortcut, createConditionalScope, SCOPE_PRIORITIES } from '../../../../core/input';
import { TreeNode } from '../types';

/**
 * Options for configuring the tree navigation behavior.
 */
interface UseTreeNavigationOptions {
    /** Accessor function that returns the node currently being navigated. */
    node: () => TreeNode<unknown>;
    /** Accessor function that returns whether the node is currently in rename/edit mode. */
    isEditing: () => boolean;
    /** Accessor function that returns whether the node has one or more child nodes. */
    hasChildren: () => boolean;
    /** Accessor function that returns the current visual expansion state of the node. */
    isExpanded: () => boolean;
    /** Optional callback invoked when the node should be selected (e.g., via Enter key). */
    onSelect?: () => void;
    /** Optional callback invoked when the node expansion state should be toggled. */
    onToggle?: (expanded: boolean) => void;
    /** Optional callback invoked to exit the rename/edit mode without saving changes. */
    onEditCancel?: () => void;
    /** Accessor function that returns whether the node currently holds keyboard focus. */
    isFocused: () => boolean;
}

/**
 * Custom hook for managing keyboard navigation and focus scopes for a tree item.
 *
 * This hook integrates with the global input system to handle shortcuts like Enter for selection,
 * ArrowRight/Left for expansion, and Escape for canceling edits. It also manages a conditional
 * 'editing' scope to prevent global shortcuts from conflicting with inline text input.
 *
 * @param {UseTreeNavigationOptions} options - The navigation and state accessors for the tree node.
 * @returns {Record<string, never>} An empty object as this hook primarily registers side-effect shortcuts.
 */
export const useTreeNavigation = (options: UseTreeNavigationOptions) => {
    // Automatically manage 'editing' scope based on editing state and focus
    // This allows global shortcuts to be shadowed when typing in the tree
    createConditionalScope(
        'editing',
        () => options.isEditing() && options.isFocused(),
        SCOPE_PRIORITIES.editing,
        true
    );

    // Keyboard navigation when NOT editing
    createShortcut({
        keys: ['Enter'],
        system: true,
        action: () => {
            if (!options.isEditing()) {
                options.onSelect?.();
            }
        },
        ignoreInputs: false,
        enabled: () => !options.isEditing() && options.isFocused()
    });

    createShortcut({
        keys: ['ArrowRight'],
        system: true,
        action: () => {
            if (options.hasChildren() && !options.isExpanded()) {
                options.onToggle?.(true);
            }
        },
        enabled: () => !options.isEditing() && options.isFocused(),
        preventDefault: true
    });

    createShortcut({
        keys: ['ArrowLeft'],
        system: true,
        action: () => {
            if (options.hasChildren() && options.isExpanded()) {
                options.onToggle?.(false);
            }
        },
        enabled: () => !options.isEditing() && options.isFocused(),
        preventDefault: true
    });

    // Edit mode specific shortcuts
    createShortcut({
        keys: ['Escape'],
        system: true,
        action: () => {
            if (options.isEditing()) {
                options.onEditCancel?.();
            }
        },
        enabled: () => options.isEditing() && options.isFocused()
    });

    return {};
};
