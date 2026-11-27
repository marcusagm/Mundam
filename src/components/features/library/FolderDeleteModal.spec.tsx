import { cleanup, fireEvent, render, screen, waitFor } from '@solidjs/testing-library';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { InputProvider } from '../../../core/input/context';
import { FolderDeleteModal } from './FolderDeleteModal';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn().mockResolvedValue(null) }));
const actions = vi.hoisted(() => ({
    reviewLocationRemoval: vi.fn(),
    removeLocation: vi.fn(),
    success: vi.fn()
}));
vi.mock('../../../core/hooks', () => ({
    useLibrary: () => actions,
    useNotification: () => ({ success: actions.success })
}));

afterEach(cleanup);
beforeEach(() => vi.resetAllMocks());

function openUnlinkDialog() {
    const onClose = vi.fn();
    render(
        () => (
            <FolderDeleteModal isOpen folderIdentifier="root" folderName="Root" onClose={onClose} />
        ),
        {
            wrapper: InputProvider
        }
    );
    return {
        onClose,
        confirm: screen.getByRole<HTMLButtonElement>('button', { name: 'Unlink Folder' })
    };
}

it('rechecks the presented impact after failure and requires a fresh confirmation', async () => {
    actions.reviewLocationRemoval
        .mockResolvedValueOnce({ annotationCount: 1, annotationRevision: 'first-review' })
        .mockResolvedValueOnce({ annotationCount: 2, annotationRevision: 'new-review' });
    actions.removeLocation
        .mockResolvedValueOnce({ success: false })
        .mockResolvedValueOnce({ success: true });
    const { onClose, confirm } = openUnlinkDialog();

    expect(confirm.disabled).toBe(true);
    await screen.findByText(/permanently removes 1 saved plugin annotation/);
    fireEvent.click(confirm);
    await screen.findByText(/permanently removes 2 saved plugin annotations/);
    expect(actions.removeLocation).toHaveBeenCalledExactlyOnceWith('root', 'first-review');
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByRole('alert').textContent).toContain('Review the current impact');

    fireEvent.click(confirm);
    await waitFor(() => expect(onClose).toHaveBeenCalledOnce());
    expect(actions.removeLocation).toHaveBeenLastCalledWith('root', 'new-review');
});

it('keeps unlink disabled when the impact cannot be loaded', async () => {
    actions.reviewLocationRemoval
        .mockRejectedValueOnce(new Error('Unavailable'))
        .mockResolvedValueOnce({ annotationCount: 0, annotationRevision: 'empty-review' });
    const { confirm } = openUnlinkDialog();
    await screen.findByRole('alert');
    expect(confirm.disabled).toBe(true);
    expect(actions.removeLocation).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: 'Retry' }));
    await waitFor(() => expect(confirm.disabled).toBe(false));
    expect(screen.queryByText(/permanently removes/)).toBeNull();
});
