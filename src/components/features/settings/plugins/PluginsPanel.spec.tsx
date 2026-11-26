import { cleanup, fireEvent, render, screen, waitFor, within } from '@solidjs/testing-library';
import { afterEach, expect, it, vi } from 'vitest';
import type {
    PluginPackageReview,
    PluginSettings,
    PluginSettingsClient
} from '../../../../lib/plugins';
import { PluginsPanel } from './PluginsPanel';

vi.mock('../../../../core/input', () => ({
    useInput: () => ({ pushScope: vi.fn(), popScope: vi.fn() }),
    SCOPE_PRIORITIES: { editing: 1 }
}));

const fingerprint = 'ab'.repeat(32);
const review: PluginPackageReview = {
    reviewId: 'opaque-review',
    pluginId: 'org.example.review',
    name: 'Review example',
    description: 'Mark references as reviewed.',
    version: '1.0.0',
    license: 'MIT',
    publisher: 'org.example',
    publisherFingerprint: fingerprint,
    iconBase64: 'iVBORw0KGgo=',
    targets: [{ platform: 'all', architecture: 'all' }],
    requiredPermissions: ['selection.read', 'assetData.own.write'],
    optionalPermissions: [],
    compatible: true,
    replacement: null
};

function renderPanel(overrides: Partial<PluginSettingsClient> = {}) {
    const client: PluginSettingsClient = {
        load: vi.fn().mockResolvedValue({ publishers: [], installed: [], safeMode: false }),
        choose: vi.fn().mockResolvedValue(null),
        trust: vi.fn().mockResolvedValue(undefined),
        revoke: vi.fn().mockResolvedValue(undefined),
        install: vi.fn(),
        discard: vi.fn().mockResolvedValue(undefined),
        reviewActivation: vi.fn(),
        enable: vi.fn().mockResolvedValue(undefined),
        disable: vi.fn().mockResolvedValue(undefined),
        reviewRollback: vi.fn(),
        remove: vi.fn().mockResolvedValue(undefined),
        ...overrides
    };
    render(() => <PluginsPanel client={client} />);
    return client;
}

afterEach(cleanup);

it('requires independent publisher confirmation and a separate revocation confirmation', async () => {
    const state: PluginSettings = { publishers: [], installed: [], safeMode: false };
    const client = renderPanel({
        load: async () => ({ ...state }),
        trust: vi.fn(async (namespace, value) => {
            state.publishers = [{ namespace, fingerprint: value, revoked: false }];
        }),
        revoke: vi.fn(async () => {
            state.publishers = [{ namespace: 'org.example', fingerprint, revoked: true }];
        })
    });
    const trust = await screen.findByRole<HTMLButtonElement>('button', { name: 'Trust publisher' });
    fireEvent.input(screen.getByRole('textbox', { name: 'Publisher namespace' }), {
        target: { value: 'org.example' }
    });
    fireEvent.input(screen.getByRole('textbox', { name: 'SHA-256 fingerprint' }), {
        target: { value: fingerprint }
    });
    expect(trust.disabled).toBe(true);
    fireEvent.click(
        screen.getByRole('checkbox', { name: 'I verified this publisher identity independently' })
    );
    expect(trust.disabled).toBe(false);
    fireEvent.click(trust);
    await screen.findByText('Publisher trusted.');
    expect(client.trust).toHaveBeenCalledWith('org.example', fingerprint);
    const revoke = await screen.findByRole<HTMLButtonElement>('button', { name: 'Revoke trust' });
    await waitFor(() => expect(revoke.disabled).toBe(false));
    fireEvent.click(revoke);
    expect(client.revoke).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Confirm revocation' }));
    await screen.findByText('Revoked');
    expect(client.revoke).toHaveBeenCalledWith('org.example', fingerprint);
});

it('reviews permissions and passes only the backend review ID to installation', async () => {
    const load = vi
        .fn()
        .mockResolvedValueOnce({ publishers: [], installed: [], safeMode: false })
        .mockResolvedValue({ publishers: [], installed: [installedPlugin], safeMode: false });
    const client = renderPanel({
        load,
        choose: vi.fn().mockResolvedValue(review),
        install: vi.fn().mockResolvedValue(installedPlugin)
    });
    fireEvent.click(await screen.findByRole('button', { name: 'Choose plugin package' }));
    await screen.findByRole('region', { name: review.name });
    expect(screen.getByText('Save its own fields on the selected asset')).toBeTruthy();
    expect(client.install).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Install disabled' }));
    await screen.findByText('Disabled');
    expect(client.install).toHaveBeenCalledExactlyOnceWith('opaque-review');
});

it('blocks incompatible packages and discards a cancelled compatible review', async () => {
    const choose = vi
        .fn()
        .mockResolvedValueOnce({ ...review, compatible: false, reviewId: null })
        .mockResolvedValueOnce(review);
    const client = renderPanel({ choose });
    const select = await screen.findByRole('button', { name: 'Choose plugin package' });
    fireEvent.click(select);
    await screen.findByText('This package is incompatible with this host.');
    expect(screen.queryByRole('button', { name: 'Install disabled' })).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Cancel review' }));
    expect(client.discard).not.toHaveBeenCalled();
    fireEvent.click(select);
    await screen.findByRole('button', { name: 'Install disabled' });
    fireEvent.click(screen.getByRole('button', { name: 'Cancel review' }));
    await waitFor(() => expect(client.discard).toHaveBeenCalledExactlyOnceWith('opaque-review'));
    expect(client.install).not.toHaveBeenCalled();
});

it('shows a single refusal beside the package picker', async () => {
    const choose = vi.fn().mockRejectedValueOnce(new Error('Untrusted publisher'));
    const client = renderPanel({ choose });
    const picker = await screen.findByRole('button', { name: 'Choose plugin package' });
    fireEvent.click(picker);
    const message = await screen.findByRole('alert');
    expect(message.textContent).toContain(
        'Choose a valid signed package from a trusted publisher.'
    );
    expect(
        within(message.closest('section') as HTMLElement).getByText('Install from file')
    ).toBeTruthy();
    expect(screen.getAllByRole('alert')).toHaveLength(1);
    expect(client.install).not.toHaveBeenCalled();
});

const installedPlugin = {
    id: review.pluginId,
    publisher: review.publisher,
    name: review.name,
    version: review.version,
    enabled: false,
    approved: false,
    packageRevision: 'revision-1',
    rollbackVersion: null
};

it('requires approval before enabling and only submits selected optional permissions', async () => {
    const state: PluginSettings = {
        safeMode: false,
        publishers: [{ namespace: review.publisher, fingerprint, revoked: false }],
        installed: [installedPlugin]
    };
    const client = renderPanel({
        load: async () => ({ ...state }),
        reviewActivation: vi.fn().mockResolvedValue({
            reviewId: 'catalog-consent',
            name: review.name,
            version: review.version,
            requiredPermissions: ['selection.read'],
            optionalPermissions: ['assetData.own.write']
        }),
        enable: vi.fn(async () => {
            state.installed = [{ ...installedPlugin, enabled: true, approved: true }];
        })
    });
    fireEvent.click(await screen.findByRole('button', { name: 'Enable in this catalog' }));
    const approve = await screen.findByRole<HTMLButtonElement>('button', {
        name: 'Approve and enable'
    });
    expect(screen.getByText('Required permissions')).toBeTruthy();
    expect(client.enable).not.toHaveBeenCalled();
    const optional = screen.getByRole('checkbox', {
        name: 'Save its own fields on the selected asset'
    });
    expect(optional.getAttribute('aria-checked')).toBe('false');
    await waitFor(() => expect(approve.disabled).toBe(false));
    fireEvent.click(optional);
    fireEvent.click(approve);
    await screen.findByText('Enabled');
    expect(client.enable).toHaveBeenCalledExactlyOnceWith('catalog-consent', [
        'assetData.own.write'
    ]);
});

it('pauses approved plugins in safe mode and allows permission revocation', async () => {
    const state: PluginSettings = {
        safeMode: true,
        publishers: [{ namespace: review.publisher, fingerprint, revoked: false }],
        installed: [{ ...installedPlugin, approved: true }]
    };
    const client = renderPanel({
        load: async () => ({ ...state }),
        disable: vi.fn(async () => {
            state.installed = [installedPlugin];
        })
    });
    await screen.findByText('Paused in safe mode');
    expect(screen.queryByRole('button', { name: 'Enable in this catalog' })).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Disable in this catalog' }));
    await screen.findByText('Disabled');
    expect(client.disable).toHaveBeenCalledExactlyOnceWith(installedPlugin.id);
});

it('offers version changes for review and submits the update and rollback tickets', async () => {
    const updated = {
        ...installedPlugin,
        version: '1.1.0',
        packageRevision: 'revision-2',
        rollbackVersion: '1.0.0'
    };
    const state: PluginSettings = {
        safeMode: false,
        publishers: [{ namespace: review.publisher, fingerprint, revoked: false }],
        installed: [installedPlugin]
    };
    const client = renderPanel({
        load: async () => ({ ...state }),
        choose: vi.fn().mockResolvedValue({
            ...review,
            reviewId: 'update-ticket',
            version: updated.version,
            replacement: {
                kind: 'update',
                currentVersion: '1.0.0',
                addedPermissions: ['assetData.own.write'],
                removedPermissions: []
            }
        }),
        reviewRollback: vi.fn().mockResolvedValue({
            ...review,
            reviewId: 'rollback-ticket',
            replacement: {
                kind: 'rollback',
                currentVersion: '1.1.0',
                addedPermissions: [],
                removedPermissions: ['assetData.own.write']
            }
        }),
        install: vi.fn(async reviewId => {
            const result =
                reviewId === 'update-ticket'
                    ? updated
                    : { ...installedPlugin, packageRevision: 'revision-3' };
            state.installed = [result];
            return result;
        })
    });
    fireEvent.click(await screen.findByRole('button', { name: 'Choose plugin package' }));
    const update = await screen.findByRole<HTMLButtonElement>('button', {
        name: 'Update and disable'
    });
    await waitFor(() => expect(update.disabled).toBe(false));
    expect(screen.getByText('New or newly required permissions')).toBeTruthy();
    expect(client.install).not.toHaveBeenCalled();
    fireEvent.click(update);
    const rollBack = await screen.findByRole<HTMLButtonElement>('button', {
        name: 'Review rollback to 1.0.0'
    });
    await waitFor(() => expect(rollBack.disabled).toBe(false));
    fireEvent.click(rollBack);
    const restore = await screen.findByRole<HTMLButtonElement>('button', {
        name: 'Restore and disable'
    });
    await waitFor(() => expect(restore.disabled).toBe(false));
    expect(screen.getByText('Removed permissions')).toBeTruthy();
    fireEvent.click(restore);
    await screen.findByText('Review example 1.0.0 is installed and disabled in all catalogs.');
    expect(client.reviewRollback).toHaveBeenCalledExactlyOnceWith(installedPlugin.id);
    expect(client.install).toHaveBeenNthCalledWith(1, 'update-ticket');
    expect(client.install).toHaveBeenNthCalledWith(2, 'rollback-ticket');
});

it('requires explicit removal confirmation for the displayed package revision', async () => {
    const state: PluginSettings = {
        safeMode: true,
        publishers: [{ namespace: review.publisher, fingerprint, revoked: true }],
        installed: [installedPlugin]
    };
    const client = renderPanel({
        load: async () => ({ ...state }),
        remove: vi.fn(async () => {
            state.installed = [];
        })
    });
    const openRemoval = await screen.findByRole('button', { name: 'Remove plugin' });
    fireEvent.click(openRemoval);
    expect(client.remove).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Cancel removal' }));
    expect(screen.queryByRole('button', { name: 'Confirm removal' })).toBeNull();
    fireEvent.click(openRemoval);
    fireEvent.click(screen.getByRole('button', { name: 'Confirm removal' }));
    await screen.findByText('No installed plugins.');
    expect(client.remove).toHaveBeenCalledExactlyOnceWith(
        installedPlugin.id,
        installedPlugin.packageRevision
    );
});
