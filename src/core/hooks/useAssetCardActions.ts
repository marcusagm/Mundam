/**
 * useAssetCardActions
 *
 * Centralizes all actions for asset cards, keeping the AssetCard component pure.
 * This hook should be used by viewport containers (VirtualMasonry, VirtualGridView)
 * to get callbacks to pass down to AssetCard components.
 */

import { useSelection } from './useSelection';
import { useViewport } from './useViewport';
import { useLibrary } from './useLibrary';
import { useTrashActions } from './useTrashActions';
import { libraryState } from '../store/library';
import { type AssetItem } from '../../types';

export interface AssetCardActions {
    /** Toggle selection for an item, optionally with multi-select or range-select */
    handleSelect: (itemId: string, modifiers: { multi: boolean; shift: boolean }) => void;
    /** Move selected items, or the focused item, to the trash. */
    handleMoveToTrash: (focusedItemId: string | null) => Promise<void>;
    /** Open item in detail/preview view */
    handleOpen: (itemId: string) => void;
    /** Check if an item is currently selected */
    isSelected: (itemId: string) => boolean;
    /** Get all currently selected IDs (for DnD) */
    getSelectedIds: () => string[];
}

export function useAssetCardActions(): AssetCardActions {
    const selection = useSelection();
    const viewport = useViewport();
    const library = useLibrary();
    const trash = useTrashActions();

    return {
        handleSelect: (itemId: string, modifiers: { multi: boolean; shift: boolean }) => {
            if (modifiers.shift) {
                const allIdentifiers = libraryState.items.map((item: AssetItem) => item.id);
                selection.selectRange(itemId, allIdentifiers);
            } else {
                selection.toggle(itemId, modifiers.multi);
            }
        },

        handleMoveToTrash: async (focusedItemId: string | null) => {
            const currentItemIdentifiers = new Set(library.items.map(item => item.id));
            const selectedItemIdentifiers = selection.selectedIds.filter(identifier =>
                currentItemIdentifiers.has(identifier)
            );
            const itemIdentifiersToTrash =
                selectedItemIdentifiers.length > 0
                    ? selectedItemIdentifiers
                    : focusedItemId && currentItemIdentifiers.has(focusedItemId)
                      ? [focusedItemId]
                      : [];

            if (itemIdentifiersToTrash.length === 0) return;

            await trash.moveToTrash(itemIdentifiersToTrash);
            selection.clear();
        },

        handleOpen: (itemId: string) => {
            viewport.openItem(itemId);
        },

        isSelected: (itemId: string) => {
            return selection.selectedIds.includes(itemId);
        },

        getSelectedIds: () => {
            return selection.selectedIds;
        }
    };
}
