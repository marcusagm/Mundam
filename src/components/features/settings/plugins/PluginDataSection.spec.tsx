import { cleanup, fireEvent, render, screen, waitFor } from '@solidjs/testing-library';
import { afterEach, expect, it, vi } from 'vitest';
import { PluginDataSection } from './PluginDataSection';

afterEach(cleanup);

const emptyStorage = () => ({ list: vi.fn(), review: vi.fn(), purge: vi.fn() });

it('handles cancellation, destination errors, and successful empty exports', async () => {
    const exportAnnotations = vi
        .fn()
        .mockResolvedValueOnce(null)
        .mockRejectedValueOnce(new Error('Private destination details'))
        .mockResolvedValueOnce(0);
    render(() => (
        <PluginDataSection exportAnnotations={exportAnnotations} storedData={emptyStorage()} />
    ));
    const button = screen.getByRole<HTMLButtonElement>('button', {
        name: 'Export plugin annotations'
    });

    fireEvent.click(button);
    await waitFor(() => expect(button.disabled).toBe(false));
    expect(screen.queryByRole('status')).toBeNull();
    expect(screen.queryByRole('alert')).toBeNull();

    fireEvent.click(button);
    expect((await screen.findByRole('alert')).textContent).toContain('Choose a writable JSON file');
    expect(screen.queryByText(/Private destination/)).toBeNull();

    fireEvent.click(button);
    await screen.findByText('Exported 0 plugin annotations.');
    expect(screen.queryByRole('alert')).toBeNull();
    expect(exportAnnotations).toHaveBeenCalledTimes(3);
});

it('requires a fresh confirmation when the reviewed rows change', async () => {
    const owner = { pluginId: 'org.example.removed', publisherFingerprint: 'ab'.repeat(32) };
    const counts = { annotations: 1, recoveryAnnotations: 2, cachedResults: 3 };
    const storedData = {
        list: vi
            .fn()
            .mockResolvedValueOnce([{ owner, counts }])
            .mockResolvedValueOnce([{ owner, counts }])
            .mockResolvedValueOnce([]),
        review: vi
            .fn()
            .mockResolvedValueOnce({ counts, revision: 'first' })
            .mockResolvedValueOnce({ counts, revision: 'fresh' }),
        purge: vi.fn().mockResolvedValueOnce(false).mockResolvedValueOnce(true)
    };
    render(() => <PluginDataSection exportAnnotations={vi.fn()} storedData={storedData} />);
    fireEvent.click(screen.getByRole('button', { name: 'Manage stored data' }));
    const review = await screen.findByRole('button', { name: 'Review data removal' });
    fireEvent.click(review);
    expect(storedData.purge).not.toHaveBeenCalled();
    fireEvent.click(await screen.findByRole('button', { name: 'Permanently remove data' }));
    await screen.findByText('Stored data changed. Review the current data before removing it.');
    expect(storedData.purge).toHaveBeenCalledExactlyOnceWith(owner, 'first');
    expect(screen.queryByRole('button', { name: 'Permanently remove data' })).toBeNull();

    const current = await waitFor(() => {
        const button = screen.getByRole<HTMLButtonElement>('button', {
            name: 'Review data removal'
        });
        expect(button.disabled).toBe(false);
        return button;
    });
    fireEvent.click(current);
    fireEvent.click(await screen.findByRole('button', { name: 'Permanently remove data' }));
    await screen.findByText('Stored plugin data removed from this catalog.');
    await screen.findByText('No stored plugin data in this catalog.');
    expect(storedData.purge).toHaveBeenLastCalledWith(owner, 'fresh');
});
