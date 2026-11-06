import { createSignal } from 'solid-js';
import { useMetadata, useNotification, useTree } from '../../../core/hooks';
import { executeLibraryOperation } from '../../../core/store/library/folderActions';
import type { DragItem } from '../../../core/dnd/dnd-core';
import type { TreeNode } from '../../ui/TreeView';
import { isFolderDescendant } from './folderHierarchy';

/** Converts the tree namespace into a database identifier.
 * @param identifier - Tree node ID.
 * @returns Catalog ID without the display namespace.
 * @example folderIdentifier('folder-example');
 */
export function folderIdentifier(identifier: string | number): string {
    return String(identifier).replace(/^folder-/, '');
}

/** Coordinates exclusive edit transitions and physical subtree drops.
 * @returns Reactive editor state and asynchronous command handlers; failed drafts remain editable.
 * @example const actions = useFolderTreeActions();
 */
export function useFolderTreeActions() {
    const metadata = useMetadata();
    const notification = useNotification();
    const tree = useTree('folders');
    const [editingId, setEditingId] = createSignal<string | null>(null);
    type EditPhase = 'idle' | 'creating' | 'editing' | 'saving' | 'cancelling';
    const [phase, setPhase] = createSignal<EditPhase>('idle');
    let draft: { id: string; operationId: string } | null = null;
    const beginEdit = (identifier: string | null): void => {
        if (phase() !== 'idle') return;
        setEditingId(identifier);
        setPhase(identifier ? 'editing' : 'idle');
    };

    /** Creates a uniquely named child and enters the same inline editor as rename. */
    const handleCreateChild = async (parentId: string): Promise<void> => {
        if (phase() !== 'idle') return;
        setPhase('creating');
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
            draft = { id: result.id, operationId: result.operationId };
            setEditingId('folder-' + result.id);
            setPhase('editing');
            if (result.warning) notification.warning('Folder Created', result.warning);
        } catch {
            notification.error('Failed to Create Folder');
        } finally {
            if (phase() === 'creating') setPhase('idle');
        }
    };

    /** Persists the inline edit and keeps the editor available when saving fails. */
    const handleRename = async (node: TreeNode, newName: string): Promise<void> => {
        // A blur fired while the editor closes (Escape) must not commit a stale edit.
        if (phase() !== 'editing' || editingId() !== String(node.id)) return;
        const name = newName.trim();
        if (!name || name === node.label) {
            draft = null;
            setPhase('idle');
            setEditingId(null);
            return;
        }
        setPhase('saving');
        try {
            const result = await executeLibraryOperation({
                kind: 'renameFolder',
                folderId: folderIdentifier(node.id),
                name
            });
            draft = null;
            setPhase('idle');
            setEditingId(null);
            if (result.warning) notification.warning('Folder monitoring', result.warning);
            notification.success('Folder Renamed', `Changed “${node.label}” to “${name}”`);
        } catch {
            notification.error('Failed to Rename Folder');
        } finally {
            if (phase() === 'saving') setPhase('editing');
        }
    };

    /** Leaves rename untouched, but discards a folder created moments ago and never confirmed. */
    const handleEditCancel = async (): Promise<void> => {
        if (phase() !== 'editing') return;
        if (!draft) {
            setEditingId(null);
            setPhase('idle');
            return;
        }
        setPhase('cancelling');
        try {
            await executeLibraryOperation({
                kind: 'cancelFolderDraft',
                folderId: draft.id,
                creationOperationId: draft.operationId
            });
            draft = null;
            setEditingId(null);
            setPhase('idle');
        } catch {
            setPhase('editing');
            notification.error(
                'Folder Kept',
                'The folder could not be discarded safely. Confirm its name or retry.'
            );
        }
    };

    /** Rejects recursive destinations before any filesystem operation is requested. */
    const isValidDrop = (item: DragItem, target: TreeNode): boolean => {
        if (target.id === 'root') return false;
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

    return {
        editingId,
        setEditingId: beginEdit,
        handleCreateChild,
        handleRename,
        handleEditCancel,
        isValidDrop,
        handleDrop
    };
}
