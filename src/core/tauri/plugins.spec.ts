import { invoke } from '@tauri-apps/api/core';
import { expect, it, vi } from 'vitest';
import { invokeCommand } from '../../lib/api';
import { desktopPluginPresentation, desktopPlugins } from './plugins';

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

it('invalidates the presentation after committed changes and respects unsubscription', async () => {
    const refresh = vi.fn();
    const unsubscribe = desktopPluginPresentation.subscribe(refresh);
    try {
        vi.mocked(invoke).mockRejectedValueOnce(new Error('Publisher revoked'));
        await expect(desktopPlugins.enable('review', [])).rejects.toThrow('Publisher revoked');
        expect(refresh).not.toHaveBeenCalled();
        vi.mocked(invoke).mockResolvedValueOnce(undefined);
        await desktopPlugins.disable('org.example.review');
        expect(refresh).toHaveBeenCalledTimes(1);
    } finally {
        unsubscribe();
    }
    vi.mocked(invoke).mockResolvedValueOnce(undefined);
    await desktopPlugins.revoke('org.example', 'ab'.repeat(32));
    expect(refresh).toHaveBeenCalledTimes(1);
});

it('rejects unrecognized inspector controls at the desktop boundary', async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
        screens: [],
        commands: [],
        sections: [
            {
                pluginId: 'org.example.review',
                pluginName: 'Review',
                sectionId: 'review',
                packageRevision: 'ab'.repeat(32),
                title: 'Review',
                fields: [{ fieldId: 'unsafe', label: 'Unknown', kind: 'html', defaultValue: false }]
            }
        ]
    });
    await expect(desktopPluginPresentation.load()).rejects.toThrow();
});
