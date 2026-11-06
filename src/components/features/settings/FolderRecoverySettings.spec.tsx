import { cleanup, fireEvent, render, screen, waitFor } from '@solidjs/testing-library';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { InputProvider } from '../../../core/input/context';
import { FolderRecoverySettings } from './FolderRecoverySettings';

const actions = vi.hoisted(() => ({ invoke: vi.fn(), restore: vi.fn() }));
vi.mock('../../../lib/api', () => ({ invokeCommand: actions.invoke }));
vi.mock('../../../core/store/library/folderActions', () => ({
    executeLibraryOperation: actions.restore
}));
afterEach(cleanup);
beforeEach(() => {
    vi.clearAllMocks();
    actions.invoke.mockImplementation((command: string) =>
        Promise.resolve(
            command === 'get_setting'
                ? String(2 * 1024 ** 3)
                : [
                      {
                          entryId: 'entry',
                          folderId: 'folder',
                          originalPath: '/root/folder',
                          createdAt: '2026-11-06',
                          byteCount: 10,
                          restorable: true
                      }
                  ]
        )
    );
    actions.restore.mockResolvedValue({ id: 'folder' });
});

describe('persistent folder recovery settings', () => {
    it('loads persisted entries, keeps a failed restore retryable, and saves a byte budget', async () => {
        render(() => <FolderRecoverySettings />, { wrapper: InputProvider });
        const budget = screen.getByLabelText('Folder recovery budget (GiB)') as HTMLInputElement;
        await waitFor(() => expect(budget.value).toBe('2'));
        expect(actions.invoke).not.toHaveBeenCalledWith(
            'folder_recovery_history',
            expect.anything()
        );
        fireEvent.click(screen.getByRole('button', { name: 'Load Folder Recovery History' }));
        await screen.findByRole('button', { name: 'Restore Folder' });
        actions.restore.mockRejectedValueOnce({ message: 'Original parent is missing' });
        fireEvent.click(screen.getByRole('button', { name: 'Restore Folder' }));
        await waitFor(() =>
            expect(screen.getByRole('alert').textContent).toContain('Original parent is missing')
        );
        fireEvent.click(screen.getByRole('button', { name: 'Restore Folder' }));
        await waitFor(() =>
            expect(screen.queryByRole('button', { name: 'Restore Folder' })).toBeNull()
        );
        expect(actions.restore).toHaveBeenLastCalledWith({
            kind: 'restoreFolder',
            folderId: 'folder'
        });
        fireEvent.input(budget, { target: { value: '3' } });
        fireEvent.click(screen.getByRole('button', { name: 'Save Recovery Budget' }));
        await waitFor(() =>
            expect(actions.invoke).toHaveBeenCalledWith('set_setting', {
                key: 'folder_recovery_max_bytes',
                value: String(3 * 1024 ** 3)
            })
        );
    });
});
