import { cleanup, fireEvent, render, screen, waitFor } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { TreeView } from './TreeView';
import { setDragItem } from '../../../core/dnd';

vi.mock('../../../lib/api', () => ({
    invokeCommand: vi.fn().mockResolvedValue(null)
}));

afterEach(() => {
    cleanup();
    setDragItem(null);
    vi.restoreAllMocks();
});
describe('root drop boundary', () => {
    it('validates external JSON and handles rejected callbacks without an unhandled promise', async () => {
        const error = new Error('Move rejected');
        const log = vi.spyOn(console, 'error').mockImplementation(() => undefined);
        const onDrop = vi.fn().mockRejectedValue(error);
        render(() => <TreeView items={[]} acceptedDragTypes={['TAG']} onDrop={onDrop} />);
        const root = screen.getByRole('tree');
        fireEvent.drop(root, { dataTransfer: { getData: () => '{broken' } });
        fireEvent.drop(root, {
            dataTransfer: { getData: () => JSON.stringify({ type: 'TAG', payload: {} }) }
        });
        expect(onDrop).not.toHaveBeenCalled();
        fireEvent.drop(root, {
            dataTransfer: {
                getData: () => JSON.stringify({ type: 'TAG', payload: { id: 'tag', name: 'Tag' } })
            }
        });
        await waitFor(() =>
            expect(log).toHaveBeenCalledWith('Tree root drop operation failed:', error)
        );
        expect(onDrop).toHaveBeenCalledWith(
            { type: 'TAG', payload: { id: 'tag', name: 'Tag' } },
            'root',
            'inside'
        );
    });
});
