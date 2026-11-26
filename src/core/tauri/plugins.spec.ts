import { invoke } from '@tauri-apps/api/core';
import { expect, it, vi } from 'vitest';
import { invokeCommand } from '../../lib/api';
import { desktopPlugins } from './plugins';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('../../lib/api', () => ({ invokeCommand: vi.fn() }));

it('lets the settings panel handle a refused package without using the global toast wrapper', async () => {
    const rejection = {
        code: 'INTERNAL_ERROR',
        message: 'Access forbidden: Plugin publisher key is not trusted'
    };
    vi.mocked(invoke).mockRejectedValueOnce(rejection);
    await expect(desktopPlugins.choose()).rejects.toEqual(rejection);
    expect(invoke).toHaveBeenCalledExactlyOnceWith('choose_plugin_package');
    expect(invokeCommand).not.toHaveBeenCalled();
});
