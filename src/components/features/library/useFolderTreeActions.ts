import { createSignal } from 'solid-js';
import { useMetadata, useNotification, useTree } from '../../../core/hooks';
import { executeLibraryOperation } from '../../../core/store/library/folderActions';
import type { DragItem } from '../../../core/dnd/dnd-core';
import type { TreeNode } from '../../ui/TreeView';
import { isFolderDescendant } from './folderPresentation';

/** Converts the tree namespace into a database identifier. */
export function folderIdentifier(identifier: string | number): string {
    return String(identifier).replace(/^folder-/, '');
}

/** Orchestrates inline folder editing and physical subtree drops. */
export function useFolderTreeActions() {
    const metadata = useMetadata();
    const notification = useNotification();
    const tree = useTree('folders');
    const [editingId, setEditingId] = createSignal<string | null>(null);
    let isRenaming = false;
    let isCreating = false;

    /** Creates a uniquely named child and enters the same inline editor as rename. */
    const handleCreateChild = async (parentId: string): Promise<void> => {
        if (isCreating || editingId() !== null) return;
        isCreating = true;
        let name = 'New Folder';
        let suffix = 1;
        while (
            metadata.locations.some(
                folder =>
                    folder.parent_id === parentId &&
                    folder.name.toLowerCase() === name.toLowerCase()
            )
        ) {
            name = 'New Folder (' + suffix++ + ')';
        }
        try {
            const result = await executeLibraryOperation({ kind: 'createFolder', parentId, name });
            tree.setExpanded('folder-' + parentId, true);
            setEditingId('folder-' + result.id);
        } catch {
            notification.error('Failed to Create Folder');
        } finally {
            isCreating = false;
        }
    };

    /** Persists the inline edit and keeps the editor available when saving fails. */
    const handleRename = async (node: TreeNode, newName: string): Promise<void> => {
        if (isRenaming) return;
        const name = newName.trim();
        if (!name || name === node.label) {
            setEditingId(null);
            return;
        }
        isRenaming = true;
        try {
            const result = await executeLibraryOperation({
                kind: 'renameFolder',
                folderId: folderIdentifier(node.id),
                name
            });
            setEditingId(null);
            if (result.warning) notification.warning('Folder monitoring', result.warning);
            notification.success('Folder Renamed', `Changed “${node.label}” to “${name}”`);
        } catch {
            notification.error('Failed to Rename Folder');
        } finally {
            isRenaming = false;
        }
    };

    /** Rejects recursive destinations before any filesystem operation is requested. */
    const isValidDrop = (item: DragItem, target: TreeNode): boolean => {
        if (item.type === 'ASSET') return true;
        if (item.type !== 'FOLDER') return false;
        const sourceId = folderIdentifier(item.payload.id);
        const source = metadata.locations.find(folder => folder.id === sourceId);
        return (
            !!source &&
            !isFolderDescendant(folderIdentifier(target.id), sourceId, metadata.locations)
        );
    };

    /** Moves assets or a complete folder into the node under the pointer. */
    const handleDrop = async (item: DragItem, targetId: string | number): Promise<void> => {
        if (targetId === 'root') return;
        const destinationId = folderIdentifier(targetId);
        const target = { id: targetId, label: '' };
        if (!isValidDrop(item, target)) return;
        try {
            let result;
            if (item.type === 'FOLDER') {
                result = await executeLibraryOperation({
                    kind: 'moveFolder',
                    folderId: folderIdentifier(item.payload.id),
                    destinationId
                });
            } else if (item.type === 'ASSET') {
                result = await executeLibraryOperation({
                    kind: 'moveAssets',
                    assetIds: item.payload.ids,
                    destinationId
                });
            }
            if (result?.warning) notification.warning('Folder monitoring', result.warning);
            tree.setExpanded(String(targetId), true);
            notification.success('Moved to Folder');
        } catch {
            notification.error('Failed to Move to Folder');
        }
    };

    return { editingId, setEditingId, handleCreateChild, handleRename, isValidDrop, handleDrop };
}
