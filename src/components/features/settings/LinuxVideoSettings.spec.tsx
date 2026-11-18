import { createSignal, untrack } from 'solid-js';
import { cleanup, fireEvent, render, screen, waitFor } from '@solidjs/testing-library';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { LinuxVideoSettings } from './LinuxVideoSettings';
import { PerformancePanel } from './PerformancePanel';
import { setPlatformForTesting } from '../../../core/input/utils/platform';
vi.mock('./PerformanceSettings', () => ({ PerformanceSettings: () => null }));

const settings = vi.hoisted(() => ({
    updateSettings: vi.fn(),
    initialize: vi.fn(),
    linuxGraphicsCompatibility: (): boolean => false,
    linuxVideoCompatibility: (): boolean => false
}));
vi.mock('../../../core/hooks', () => ({ useSettings: () => settings }));
afterEach(() => {
    cleanup();
    setPlatformForTesting(null);
});
beforeEach(() => {
    vi.resetAllMocks();
    settings.initialize.mockResolvedValue(undefined);
    settings.linuxVideoCompatibility = () => false;
});

describe('Linux video compatibility preference', () => {
    it('persists before acknowledging the mode and explains restart', async () => {
        const [saved, setSaved] = createSignal(false);
        settings.linuxVideoCompatibility = saved;
        let complete!: () => void;
        settings.updateSettings.mockImplementation(
            () =>
                new Promise(resolve => {
                    complete = () => {
                        setSaved(true);
                        resolve({ success: true });
                    };
                })
        );
        const view = render(() => <LinuxVideoSettings />);
        const selector = view.getByRole('combobox', { name: 'Video mode:' });
        await waitFor(() => expect((selector as HTMLButtonElement).disabled).toBe(false));
        fireEvent.click(selector);
        fireEvent.click(screen.getByRole('option', { name: 'Compatibility' }));
        expect(settings.updateSettings).toHaveBeenCalledWith({ linuxVideoCompatibility: true });
        expect((selector as HTMLButtonElement).disabled).toBe(true);
        expect(untrack(saved)).toBe(false);
        complete();
        await waitFor(() =>
            expect(view.getByRole('status').textContent).toContain('Restart Mundam')
        );
        expect(untrack(saved)).toBe(true);
    });

    it('keeps the selected mode after persistence failure without leaking the error', async () => {
        settings.updateSettings.mockResolvedValue({ success: false, error: { message: 'SECRET' } });
        const view = render(() => <LinuxVideoSettings />);
        const selector = view.getByRole('combobox', { name: 'Video mode:' });
        await waitFor(() => expect((selector as HTMLButtonElement).disabled).toBe(false));
        fireEvent.click(selector);
        fireEvent.click(screen.getByRole('option', { name: 'Compatibility' }));
        await waitFor(() =>
            expect(view.getByRole('status').textContent).toContain('Could not save')
        );
        expect(selector.textContent).toContain('Normal');
        expect(view.container.textContent).not.toContain('SECRET');
    });
});

describe('video preference platform visibility', () => {
    it.each(['mac', 'windows'] as const)('hides Linux-specific controls on %s', platform => {
        setPlatformForTesting(platform);
        const view = render(() => <PerformancePanel />);
        expect(view.queryByRole('combobox', { name: 'Video mode:' })).toBeNull();
    });
    it('offers normal mode by default on Linux', () => {
        setPlatformForTesting('linux');
        const view = render(() => <PerformancePanel />);
        expect(view.getByRole('combobox', { name: 'Video mode:' }).textContent).toContain('Normal');
    });
});

it('does not allow a preference write when loading settings fails', async () => {
    settings.initialize.mockRejectedValue(new Error('SECRET'));
    const view = render(() => <LinuxVideoSettings />);
    await waitFor(() => expect(view.getByRole('status').textContent).toContain('Could not load'));
    expect(
        (view.getByRole('combobox', { name: 'Video mode:' }) as HTMLButtonElement).disabled
    ).toBe(true);
    expect(settings.updateSettings).not.toHaveBeenCalled();
    expect(view.container.textContent).not.toContain('SECRET');
});
