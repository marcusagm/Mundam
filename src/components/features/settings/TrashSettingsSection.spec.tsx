import { cleanup, fireEvent, render, screen, waitFor } from '@solidjs/testing-library';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { InputProvider } from '../../../core/input/context';
import { TrashSettingsSection } from './TrashSettingsSection';

const actions = vi.hoisted(() => ({ invoke: vi.fn(), openPath: vi.fn(), error: vi.fn() }));
vi.mock('../../../lib/api', () => ({ invokeCommand: actions.invoke }));
vi.mock('@tauri-apps/plugin-opener', () => ({ openPath: actions.openPath }));
vi.mock('../../../core/hooks', () => ({
    useNotification: () => ({ error: actions.error }),
    useMetadata: () => ({ stats: { trash_assets: 0 } })
}));
afterEach(cleanup);
beforeEach(() => {
    vi.clearAllMocks();
    actions.invoke.mockImplementation((command: string) =>
        Promise.resolve(command === 'folder_recovery_directory' ? '/app/deleted-folders' : null)
    );
    actions.openPath.mockResolvedValue(undefined);
});

describe('advanced folder recovery settings', () => {
    it('opens storage only after the explicit settings action', async () => {
        render(() => <TrashSettingsSection />, { wrapper: InputProvider });
        expect(actions.openPath).not.toHaveBeenCalled();
        expect(actions.invoke).not.toHaveBeenCalledWith('folder_recovery_directory');
        fireEvent.click(screen.getByRole('button', { name: 'Open Folder Recovery' }));
        await waitFor(() => expect(actions.openPath).toHaveBeenCalledWith('/app/deleted-folders'));
        expect(screen.getByText(/Manually changing these files can prevent/)).toBeTruthy();
    });
});
