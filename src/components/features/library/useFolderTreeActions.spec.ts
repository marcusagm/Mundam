import { createRoot } from 'solid-js';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useFolderTreeActions } from './useFolderTreeActions';

const actions = vi.hoisted(() => ({
    execute: vi.fn(),
    expand: vi.fn(),
    error: vi.fn(),
    success: vi.fn(),
    warning: vi.fn()
}));
vi.mock('../../../core/store/library/folderActions', () => ({
    executeLibraryOperation: actions.execute
}));
vi.mock('../../../core/hooks', () => ({
    useMetadata: () => ({
        locations: [
            { id: 'source', parent_id: null, name: 'Source' },
            { id: 'child', parent_id: 'source', name: 'Child' },
            { id: 'destination', parent_id: null, name: 'Destination' }
        ]
    }),
    useNotification: () => actions,
    useTree: () => ({ setExpanded: actions.expand })
}));
beforeEach(() => {
    vi.clearAllMocks();
    actions.execute.mockResolvedValue({ id: 'source', path: '/destination/source' });
});

describe('folder tree actions', () => {
    it('creates a child and opens its inline editor after the authoritative refresh', async () => {
        actions.execute.mockResolvedValueOnce({ id: 'created', path: '/source/New Folder' });
        await createRoot(async dispose => {
            try {
                const tree = useFolderTreeActions();
                await tree.handleCreateChild('source');
                expect(actions.execute).toHaveBeenCalledWith({
                    kind: 'createFolder',
                    parentId: 'source',
                    name: 'New Folder'
                });
                expect(actions.expand).toHaveBeenCalledWith('folder-source', true);
                expect(tree.editingId()).toBe('folder-created');
            } finally {
                dispose();
            }
        });
    });
    it('moves a linked folder into another folder and expands the destination', async () => {
        await createRoot(async dispose => {
            try {
                const tree = useFolderTreeActions();
                await tree.handleDrop(
                    {
                        type: 'FOLDER',
                        payload: { id: 'folder-source', name: 'Source', path: '/source' }
                    },
                    'folder-destination'
                );
                expect(actions.execute).toHaveBeenCalledWith({
                    kind: 'moveFolder',
                    folderId: 'source',
                    destinationId: 'destination'
                });
                expect(actions.expand).toHaveBeenCalledWith('folder-destination', true);
            } finally {
                dispose();
            }
        });
    });
    it('rejects self and descendant drops before invoking IPC', async () => {
        await createRoot(async dispose => {
            try {
                const tree = useFolderTreeActions();
                const item = {
                    type: 'FOLDER' as const,
                    payload: { id: 'folder-source', name: 'Source', path: '/source' }
                };
                await tree.handleDrop(item, 'folder-source');
                await tree.handleDrop(item, 'folder-child');
                expect(actions.execute).not.toHaveBeenCalled();
            } finally {
                dispose();
            }
        });
    });
    it('renames inline and preserves edit mode after a failed save', async () => {
        await createRoot(async dispose => {
            try {
                const tree = useFolderTreeActions();
                tree.setEditingId('folder-child');
                actions.execute.mockRejectedValueOnce(new Error('collision'));
                await tree.handleRename({ id: 'folder-child', label: 'Child' }, 'Renamed');
                expect(tree.editingId()).toBe('folder-child');
                await tree.handleRename({ id: 'folder-child', label: 'Child' }, 'Renamed');
                expect(tree.editingId()).toBeNull();
                expect(actions.execute).toHaveBeenLastCalledWith({
                    kind: 'renameFolder',
                    folderId: 'child',
                    name: 'Renamed'
                });
            } finally {
                dispose();
            }
        });
    });
    it('deletes a just-created folder when its editor is cancelled', async () => {
        actions.execute.mockResolvedValueOnce({ id: 'created', path: '/source/New Folder' });
        await createRoot(async dispose => {
            try {
                const tree = useFolderTreeActions();
                await tree.handleCreateChild('source');
                await tree.handleEditCancel();
                expect(tree.editingId()).toBeNull();
                expect(actions.execute).toHaveBeenLastCalledWith({
                    kind: 'deleteFolder',
                    folderId: 'created'
                });
                await tree.handleEditCancel();
                expect(actions.execute).toHaveBeenCalledTimes(2);
            } finally {
                dispose();
            }
        });
    });
    it('keeps a renamed folder when editing is cancelled afterwards and ignores stale blurs', async () => {
        await createRoot(async dispose => {
            try {
                const tree = useFolderTreeActions();
                await tree.handleRename({ id: 'folder-child', label: 'Child' }, 'Renamed');
                expect(actions.execute).not.toHaveBeenCalled();
            } finally {
                dispose();
            }
        });
    });
});
