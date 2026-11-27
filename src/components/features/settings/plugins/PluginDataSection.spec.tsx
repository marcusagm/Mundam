import { cleanup, fireEvent, render, screen, waitFor } from '@solidjs/testing-library';
import { afterEach, expect, it, vi } from 'vitest';
import { PluginDataSection } from './PluginDataSection';

afterEach(cleanup);

it('handles cancellation, destination errors, and successful empty exports', async () => {
    const exportAnnotations = vi
        .fn()
        .mockResolvedValueOnce(null)
        .mockRejectedValueOnce(new Error('Private destination details'))
        .mockResolvedValueOnce(0);
    render(() => <PluginDataSection exportAnnotations={exportAnnotations} />);
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
