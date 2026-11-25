import { cleanup, fireEvent, render, screen, waitFor } from '@solidjs/testing-library';
import { afterEach, expect, it, vi } from 'vitest';
import { IntegrationsPanel } from './IntegrationsPanel';

afterEach(cleanup);

it('starts the API only on explicit action and removes connection details when stopped', async () => {
    const endpoint = {
        url: 'https://127.0.0.1:49152',
        publicKeySha256: 'public-fingerprint',
        certificateDer: 'public-certificate'
    };
    const status = vi.fn().mockResolvedValue({ available: true, endpoint: null });
    const start = vi.fn().mockResolvedValue(endpoint);
    const stop = vi.fn().mockResolvedValue(undefined);
    const pairing = {
        copyInvitation: vi.fn(),
        status: vi
            .fn()
            .mockResolvedValue({ catalogId: 'catalog', pending: [], clients: [], roots: [] }),
        approve: vi.fn(),
        reject: vi.fn(),
        revoke: vi.fn()
    };
    render(() => <IntegrationsPanel externalImport={{ status, start, stop, pairing }} />);
    const enable = await screen.findByRole('button', { name: 'Enable API' });
    expect(start).not.toHaveBeenCalled();
    fireEvent.click(enable);
    await screen.findByText(endpoint.url);
    expect(screen.getByText(endpoint.publicKeySha256)).toBeDefined();
    fireEvent.click(screen.getByRole('button', { name: 'Disable API' }));
    await screen.findByRole('button', { name: 'Enable API' });
    expect(screen.queryByText(endpoint.url)).toBeNull();
    expect(stop).toHaveBeenCalledOnce();
});

it('requires an explicit scope and root before approving an importer', async () => {
    const endpoint = {
        url: 'https://127.0.0.1:49152',
        publicKeySha256: 'digest',
        certificateDer: 'certificate'
    };
    const status = vi.fn().mockResolvedValue({ available: true, endpoint });
    const catalog = {
        catalogId: 'catalog',
        pending: [
            { id: 'browser', clientName: 'Browser', requestedScopes: ['folders.read', 'tags.read'] }
        ],
        clients: [{ id: 'old', clientName: 'Old browser', scopes: ['tags.read'], rootIds: [] }],
        roots: [{ id: 'refs', name: 'References', path: '/references' }]
    };
    const pairing = {
        copyInvitation: vi.fn().mockResolvedValue(undefined),
        status: vi.fn().mockResolvedValue(catalog),
        approve: vi.fn().mockResolvedValue(undefined),
        reject: vi.fn().mockResolvedValue(undefined),
        revoke: vi.fn().mockResolvedValue(undefined)
    };
    render(() => (
        <IntegrationsPanel externalImport={{ status, start: vi.fn(), stop: vi.fn(), pairing }} />
    ));
    await screen.findByText('Browser');
    const approve = screen.getByRole<HTMLButtonElement>('button', { name: 'Approve' });
    expect(approve.disabled).toBe(true);
    fireEvent.click(screen.getByRole('checkbox', { name: 'Read selected folder trees' }));
    expect(approve.disabled).toBe(true);
    fireEvent.click(screen.getByRole('checkbox', { name: /References/ }));
    fireEvent.click(approve);
    await screen.findByText(/Request approved/);
    expect(pairing.approve).toHaveBeenCalledWith({
        requestId: 'browser',
        catalogId: 'catalog',
        scopes: ['folders.read'],
        rootIds: ['refs']
    });
    const revoke = screen.getByRole<HTMLButtonElement>('button', { name: 'Revoke access' });
    await waitFor(() => expect(revoke.disabled).toBe(false));
    fireEvent.click(revoke);
    await screen.findByText(/Application access revoked/);
    expect(pairing.revoke).toHaveBeenCalledWith('catalog', 'old');
});
