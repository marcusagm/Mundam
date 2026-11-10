import { cleanup, render } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ShutdownScreen } from './ShutdownScreen';

vi.mock('../../../core/shutdown/useShutdownProgress', () => ({
    useShutdownProgress: () => ({
        unavailable: () => false,
        message: () => 'Stopping background activity',
        isSlow: () => false,
        elapsedSeconds: () => 0
    })
}));

afterEach(cleanup);

describe('shutdown presentation', () => {
    it('provides an indeterminate progress indicator and accessible status', () => {
        const view = render(() => <ShutdownScreen />);
        expect(view.getByRole('heading', { name: 'Closing Mundam' })).toBeTruthy();
        const progress = view.getByRole('progressbar', { name: 'Closing Mundam' });
        expect(progress.hasAttribute('aria-valuenow')).toBe(false);
        expect(progress.getAttribute('aria-valuetext')).toBe('Stopping background activity');
        expect(view.getByRole('status').textContent).toBe('Stopping background activity');
        expect(view.queryByRole('button')).toBeNull();
    });
});
