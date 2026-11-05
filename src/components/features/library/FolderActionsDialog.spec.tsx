import { cleanup, fireEvent, render, screen, waitFor } from '@solidjs/testing-library';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { FolderActionsDialog } from './FolderActionsDialog';
import { InputProvider } from '../../../core/input/context';
import type { JSX } from 'solid-js';

const renderWithInput = (component: () => JSX.Element) =>
    render(component, { wrapper: InputProvider });
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn().mockResolvedValue(null) }));

const operations = vi.hoisted(() => ({ execute: vi.fn() }));
vi.mock('../../../core/store/library/folderActions', () => ({
    executeLibraryOperation: operations.execute
}));
vi.mock('../../../core/hooks', () => ({
    useMetadata: () => ({
        locations: [
            { id: 'root', parent_id: null, name: 'Root', path: '/root' },
            { id: 'child', parent_id: 'root', name: 'Child', path: '/root/child' },
            { id: 'nested', parent_id: 'child', name: 'Nested', path: '/root/child/nested' }
        ]
    })
}));
vi.mock('@tauri-apps/plugin-opener', () => ({ openPath: vi.fn() }));
afterEach(cleanup);
beforeEach(() => {
    operations.execute.mockReset();
    operations.execute.mockResolvedValue({ path: '/root/created' });
});

describe('managed library dialogs', () => {
    it('keeps a failed rename open with the entered name and allows retry', async () => {
        operations.execute.mockRejectedValueOnce({ message: 'Destination already exists' });
        const onClose = vi.fn();
        renderWithInput(() => (
            <FolderActionsDialog
                target={{ action: 'renameAsset', assetIds: ['asset'], initialName: 'Original' }}
                onClose={onClose}
            />
        ));
        const input = screen.getByRole('textbox', { name: 'Name' });
        fireEvent.input(input, { target: { value: 'New name' } });
        fireEvent.submit(input.closest('form')!);
        await waitFor(() =>
            expect(screen.getByRole('alert').textContent).toContain('Destination already exists')
        );
        expect(onClose).not.toHaveBeenCalled();
        expect((input as HTMLInputElement).value).toBe('New name');
        fireEvent.submit(input.closest('form')!);
        await waitFor(() => expect(onClose).toHaveBeenCalledOnce());
        expect(operations.execute).toHaveBeenLastCalledWith({
            kind: 'renameAsset',
            assetId: 'asset',
            name: 'New name'
        });
    });

    it('offers only destinations outside the selected subtree', () => {
        renderWithInput(() => (
            <FolderActionsDialog
                target={{ action: 'moveFolder', folderId: 'child' }}
                onClose={() => undefined}
            />
        ));
        expect(screen.getByRole('treeitem', { name: /Root/ })).toBeTruthy();
        expect(screen.queryByRole('treeitem', { name: /Child/ })).toBeNull();
        expect(screen.queryByRole('treeitem', { name: /Nested/ })).toBeNull();
        expect(screen.queryByRole('combobox')).toBeNull();
    });

    it('moves selected assets to the folder selected in the tree', async () => {
        renderWithInput(() => (
            <FolderActionsDialog
                target={{ action: 'moveAssets', assetIds: ['asset'] }}
                onClose={() => undefined}
            />
        ));
        const confirm = screen.getByRole('button', { name: 'Confirm' });
        expect((confirm as HTMLButtonElement).disabled).toBe(true);
        fireEvent.click(screen.getByRole('treeitem', { name: /Child/ }));
        fireEvent.click(confirm);
        await waitFor(() =>
            expect(operations.execute).toHaveBeenCalledWith({
                kind: 'moveAssets',
                assetIds: ['asset'],
                destinationId: 'child'
            })
        );
    });
});
