import { cleanup, fireEvent, render, screen } from '@solidjs/testing-library';
import { afterEach, expect, it, vi } from 'vitest';
import type { ParentProps } from 'solid-js';
import { FoldersPanel } from './FoldersPanel';
import { SettingsModal } from './SettingsModal';

vi.mock('../../../core/hooks', () => ({ useMetadata: () => ({ locations: [{}] }) }));
vi.mock('../../ui', () => ({ Modal: (properties: ParentProps) => <>{properties.children}</> }));
vi.mock('./GeneralPanel', () => ({ GeneralPanel: () => null }));
vi.mock('./AppearancePanel', () => ({ AppearancePanel: () => null }));
vi.mock('./PerformancePanel', () => ({ PerformancePanel: () => null }));
vi.mock('./StoragePanel', () => ({ StoragePanel: () => null }));
vi.mock('./KeyboardShortcutsPanel', () => ({ KeyboardShortcutsPanel: () => null }));
vi.mock('./DuplicatesPanel', () => ({ DuplicatesPanel: () => null }));
vi.mock('./AboutPanel', () => ({ AboutPanel: () => null }));
vi.mock('./plugins/PluginsPanel', () => ({ PluginsPanel: () => null }));
vi.mock('../library/FolderDestinationPicker', () => ({
    FolderDestinationPicker: (properties: {
        value: string;
        isDisabled: boolean;
        onSelect(folderId: string): void;
    }) => (
        <button disabled={properties.isDisabled} onClick={() => properties.onSelect('references')}>
            {properties.value || 'Choose references'}
        </button>
    )
}));
afterEach(cleanup);

it('opens the folder settings from the settings sidebar', async () => {
    const load = vi.fn().mockResolvedValue({ catalogId: 'catalog', folderId: null });
    render(() => (
        <SettingsModal
            isOpen
            onClose={() => undefined}
            importPreferences={{ load, save: vi.fn() }}
            plugins={{
                load: vi.fn(),
                trust: vi.fn(),
                revoke: vi.fn(),
                choose: vi.fn(),
                install: vi.fn(),
                discard: vi.fn(),
                reviewActivation: vi.fn(),
                enable: vi.fn(),
                disable: vi.fn()
            }}
            externalImport={{
                status: vi.fn(),
                start: vi.fn(),
                stop: vi.fn(),
                pairing: {
                    copyInvitation: vi.fn(),
                    status: vi.fn(),
                    approve: vi.fn(),
                    reject: vi.fn(),
                    revoke: vi.fn()
                }
            }}
        />
    ));
    const folders = screen.getByRole('button', { name: 'Folders' });
    expect(screen.getByRole('button', { name: 'Plugins' })).toBeTruthy();
    fireEvent.click(folders);
    expect(folders.getAttribute('aria-current')).toBe('page');
    await screen.findByRole('button', { name: 'Choose references' });
    expect(load).toHaveBeenCalledTimes(1);
});

it('loads, saves and clears the default through a replaceable client', async () => {
    const catalogId = 'catalog';
    const load = vi
        .fn()
        .mockRejectedValueOnce(new Error('Unavailable'))
        .mockResolvedValueOnce({ catalogId, folderId: null });
    const save = vi.fn(async (_catalogId: string, folderId: string | null) => ({
        catalogId,
        folderId
    }));
    render(() => <FoldersPanel importPreferences={{ load, save }} />);
    await screen.findByRole('alert');
    expect(
        screen
            .getByRole('heading', { name: 'Default import folder' })
            .closest('section')
            ?.classList.contains('section-group')
    ).toBe(true);
    fireEvent.click(screen.getByRole('button', { name: 'Retry' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Choose references' }));
    await screen.findByText('Default import folder saved.');
    expect(save).toHaveBeenLastCalledWith(catalogId, 'references');
    expect(screen.getByRole('button', { name: 'references' })).toBeDefined();
    fireEvent.click(screen.getByRole('button', { name: 'Clear default' }));
    await screen.findByText('Default import folder cleared.');
    expect(save).toHaveBeenLastCalledWith(catalogId, null);
});
