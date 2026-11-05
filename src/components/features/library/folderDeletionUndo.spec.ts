import { beforeEach, describe, expect, it, vi } from 'vitest';
import { showFolderDeletionUndo } from './folderDeletionUndo';

const actions = vi.hoisted(() => ({ execute: vi.fn(), success: vi.fn(), error: vi.fn() }));
vi.mock('../../../core/store/library/folderActions', () => ({
    executeLibraryOperation: actions.execute
}));
vi.mock('../../ui/Sonner/state', () => ({ toast: actions }));
beforeEach(() => {
    vi.clearAllMocks();
    actions.execute.mockResolvedValue({ id: 'folder', path: '/folder' });
});

describe('folder deletion undo', () => {
    it('offers Undo instead of opening storage and restores the original folder identity', async () => {
        showFolderDeletionUndo('folder');
        const options = actions.success.mock.calls[0][1];
        expect(options.action.label).toBe('Undo');
        expect(options.duration).toBe(0);
        options.action.onClick();
        await vi.waitFor(() => expect(actions.success).toHaveBeenCalledWith('Folder restored'));
        expect(actions.execute).toHaveBeenCalledWith({ kind: 'restoreFolder', folderId: 'folder' });
    });
    it('keeps an actionable retry when restoration fails', async () => {
        actions.execute.mockRejectedValueOnce(new Error('destination occupied'));
        showFolderDeletionUndo('folder');
        actions.success.mock.calls[0][1].action.onClick();
        await vi.waitFor(() => expect(actions.error).toHaveBeenCalled());
        const retry = actions.error.mock.calls[0][1].action;
        expect(retry.label).toBe('Try Undo Again');
        retry.onClick();
        await vi.waitFor(() => expect(actions.success).toHaveBeenCalledWith('Folder restored'));
        expect(actions.execute).toHaveBeenCalledTimes(2);
    });
});
