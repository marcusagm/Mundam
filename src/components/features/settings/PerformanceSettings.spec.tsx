import { cleanup, fireEvent, render, screen, waitFor } from '@solidjs/testing-library';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { PerformanceSettings } from './PerformanceSettings';

const settings = vi.hoisted(() => ({
    updateSettings: vi.fn(),
    processingConcurrency: () => 4,
    thumbnailThreads: () => 2,
    indexerConcurrencyLimit: () => 200
}));
vi.mock('../../../core/hooks', () => ({ useSettings: () => settings }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn().mockResolvedValue(null) }));
afterEach(cleanup);
beforeEach(() => {
    vi.resetAllMocks();
    settings.updateSettings.mockResolvedValue({ success: true });
});

describe('performance settings', () => {
    it('names interactive selectors and acknowledges live changes only after saving', async () => {
        let complete!: (result: { success: true }) => void;
        settings.updateSettings.mockImplementation(
            () =>
                new Promise<{ success: true }>(resolve => {
                    complete = resolve;
                })
        );
        const view = render(() => <PerformanceSettings />);
        const shared = view.getByRole('combobox', { name: 'Shared processing limit:' });
        fireEvent.click(shared);
        fireEvent.click(screen.getByRole('option', { name: '1 operation' }));
        expect(settings.updateSettings).toHaveBeenCalledWith({ processingConcurrency: 1 });
        expect((shared as HTMLButtonElement).disabled).toBe(true);
        expect(view.getByRole('status').textContent).toContain('Saving');
        complete({ success: true });
        await waitFor(() => expect((shared as HTMLButtonElement).disabled).toBe(false));
        expect(view.getByRole('status').textContent).toContain(
            'Current operations finish normally'
        );
    });

    it('explains that only the indexer requires restarting', async () => {
        const view = render(() => <PerformanceSettings />);
        expect(view.getByText(/Media limits apply without restarting/)).toBeTruthy();
        fireEvent.click(view.getByRole('combobox', { name: 'Indexer concurrency:' }));
        fireEvent.click(screen.getByRole('option', { name: '100 (Balanced)' }));
        await waitFor(() =>
            expect(view.getByRole('status').textContent).toContain('Restart the app')
        );
        expect(settings.updateSettings).toHaveBeenCalledWith({ indexerConcurrencyLimit: 100 });
    });

    it('reports save failure inline without exposing backend error details', async () => {
        settings.updateSettings.mockResolvedValue({ success: false, error: { message: 'SECRET' } });
        const view = render(() => <PerformanceSettings />);
        fireEvent.click(view.getByRole('combobox', { name: 'Thumbnail limit:' }));
        fireEvent.click(screen.getByRole('option', { name: '1 operation' }));
        await waitFor(() =>
            expect(view.getByRole('status').textContent).toContain('Could not save')
        );
        expect(view.container.textContent).not.toContain('SECRET');
        expect(
            (view.getByRole('combobox', { name: 'Thumbnail limit:' }) as HTMLButtonElement).disabled
        ).toBe(false);
    });
});
