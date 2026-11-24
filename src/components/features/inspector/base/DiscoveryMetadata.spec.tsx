import { cleanup, fireEvent, render, screen, waitFor } from '@solidjs/testing-library';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createStore } from 'solid-js/store';
import { DiscoveryMetadata } from './DiscoveryMetadata';
import { InputProvider } from '../../../../core/input/context';
import { type AssetItem } from '../../../../types';

const actions = vi.hoisted(() => ({ updateItemDiscovery: vi.fn() }));
vi.mock('../../../../core/store/library/itemActions', () => ({
    itemActions: actions,
    setRefreshAssetsHandler: vi.fn()
}));
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: vi.fn().mockResolvedValue(undefined) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

afterEach(cleanup);
beforeEach(() => {
    actions.updateItemDiscovery.mockReset();
});

function renderInspector(sourceUrl: string | null = null) {
    const [item, setItem] = createStore({
        id: 'asset',
        keywords: [],
        source_url: sourceUrl
    } as unknown as AssetItem);
    actions.updateItemDiscovery.mockImplementation(async (_identifier, changes) => {
        if (changes.keywords) setItem('keywords', changes.keywords);
        if (changes.sourceUrl !== undefined) setItem('source_url', changes.sourceUrl || null);
        return { ...item };
    });
    return render(() => (
        <InputProvider>
            <DiscoveryMetadata item={item} />
        </InputProvider>
    ));
}

describe('asset discovery metadata', () => {
    it('commits pasted comma-separated keywords and Enter without duplicates', async () => {
        renderInspector();
        const input = screen.getByRole('combobox', { name: 'Asset keywords' });
        fireEvent.input(input, { target: { value: ' nature, Nature, concept art,' } });
        await waitFor(() =>
            expect(actions.updateItemDiscovery).toHaveBeenCalledWith('asset', {
                keywords: ['nature', 'concept art']
            })
        );
        fireEvent.input(input, { target: { value: 'lighting' } });
        fireEvent.keyDown(input, { key: 'Enter' });
        await waitFor(() =>
            expect(actions.updateItemDiscovery).toHaveBeenLastCalledWith('asset', {
                keywords: ['nature', 'concept art', 'lighting']
            })
        );
    });

    it('cancels URL changes, saves a link and clears it', async () => {
        renderInspector('https://example.com/original');
        fireEvent.click(screen.getByRole('button', { name: 'Edit source URL' }));
        expect(
            (screen.getByRole('textbox', { name: 'Source URL' }) as HTMLInputElement).value
        ).toBe('https://example.com/original');
        fireEvent.input(screen.getByRole('textbox', { name: 'Source URL' }), {
            target: { value: 'https://example.com/new' }
        });
        fireEvent.click(screen.getByRole('button', { name: 'Cancel source URL editing' }));
        expect(actions.updateItemDiscovery).not.toHaveBeenCalled();
        fireEvent.click(screen.getByRole('button', { name: 'Edit source URL' }));
        fireEvent.input(screen.getByRole('textbox', { name: 'Source URL' }), {
            target: { value: '' }
        });
        fireEvent.click(screen.getByRole('button', { name: 'Save source URL' }));
        await waitFor(() => expect(screen.getByText('No source link added yet.')).toBeTruthy());
        fireEvent.click(screen.getByRole('button', { name: 'Edit source URL' }));
        fireEvent.input(screen.getByRole('textbox', { name: 'Source URL' }), {
            target: { value: 'https://example.com/new' }
        });
        fireEvent.keyDown(screen.getByRole('textbox', { name: 'Source URL' }), { key: 'Enter' });
        await waitFor(() =>
            expect(screen.getByRole('link').getAttribute('href')).toBe('https://example.com/new')
        );
    });

    it('rejects unsafe URLs and keeps the draft after persistence fails', async () => {
        renderInspector();
        fireEvent.click(screen.getByRole('button', { name: 'Edit source URL' }));
        const input = screen.getByRole('textbox', { name: 'Source URL' });
        fireEvent.input(input, { target: { value: 'javascript:alert(1)' } });
        fireEvent.click(screen.getByRole('button', { name: 'Save source URL' }));
        expect(actions.updateItemDiscovery).not.toHaveBeenCalled();
        expect(screen.getByRole('alert').textContent).toContain('HTTP or HTTPS');
        actions.updateItemDiscovery.mockRejectedValueOnce(new Error('offline'));
        fireEvent.input(input, { target: { value: 'https://example.com' } });
        fireEvent.click(screen.getByRole('button', { name: 'Save source URL' }));
        await waitFor(() =>
            expect(screen.getByRole('alert').textContent).toContain('could not be saved')
        );
        expect((input as HTMLInputElement).value).toBe('https://example.com');
    });
});
