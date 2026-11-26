import { cleanup, fireEvent, render, screen, waitFor } from '@solidjs/testing-library';
import { createSignal, Show } from 'solid-js';
import { afterEach, expect, it, vi } from 'vitest';
import { PluginScreensMenu } from '../components/layout/TitleBar/PluginScreensMenu';
import { InputProvider } from '../core/input';
import { createPluginPresentation } from '../core/hooks/createPluginPresentation';
import {
    pluginScreenRoute,
    type PluginPresentation,
    type PluginScreen
} from '../lib/pluginPresentation';
import { createApplicationNavigation } from './createApplicationNavigation';
import { PluginScreenView } from './PluginScreenView';

vi.mock('../lib/api', () => ({ invokeCommand: vi.fn().mockResolvedValue(null) }));
vi.mock('../components/layout/GlobalStatusbar', () => ({ GlobalStatusbar: () => null }));

const guide: PluginScreen = {
    pluginId: 'org.example.review',
    pluginName: 'Reference review',
    screenId: 'guide',
    title: 'Review guide',
    blocks: [
        { kind: 'paragraph', text: '<b>Plain text</b>' },
        { kind: 'list', items: ['Choose an asset.', 'Open the Inspector.'] }
    ]
};

afterEach(cleanup);

it('opens a qualified screen, hides it during revalidation, and returns to the gallery after revocation', async () => {
    const otherGuide = { ...guide, pluginId: 'org.other.review', pluginName: 'Other review' };
    const [catalog, setCatalog] = createSignal('catalog-a');
    let invalidate = () => {};
    let resolveCurrent!: (value: PluginPresentation) => void;
    const pending = new Promise<PluginPresentation>(resolve => {
        resolveCurrent = resolve;
    });
    const enabled = { sections: [], screens: [guide, otherGuide], commands: [] };
    const load = vi
        .fn()
        .mockResolvedValueOnce(enabled)
        .mockReturnValueOnce(pending)
        .mockResolvedValueOnce(enabled)
        .mockResolvedValueOnce({ sections: [], screens: [], commands: [] });
    const client = {
        load,
        subscribe: (onChange: () => void) => {
            invalidate = onChange;
            return vi.fn();
        }
    };

    render(() => {
        const presentation = createPluginPresentation(() => client, catalog);
        const navigation = createApplicationNavigation(presentation.snapshot, () =>
            Boolean(catalog())
        );
        return (
            <InputProvider>
                <PluginScreensMenu
                    screens={navigation.screens()}
                    onNavigate={navigation.navigate}
                />
                <output aria-label="Active view">{navigation.activeView()}</output>
                <Show when={navigation.activeScreen()} keyed>
                    {active => (
                        <PluginScreenView
                            header={null}
                            screen={active}
                            focusHeading
                            onBack={() => navigation.navigate('gallery')}
                        />
                    )}
                </Show>
            </InputProvider>
        );
    });

    const openGuide = async () => {
        fireEvent.click(await screen.findByRole('button', { name: 'Plugin screens' }));
        fireEvent.click(
            await screen.findByRole('menuitem', { name: 'Review guide · Reference review' })
        );
        expect(await screen.findByRole('heading', { name: 'Review guide' })).toBe(
            document.activeElement
        );
    };
    await openGuide();
    expect(screen.getByLabelText('Active view').textContent).toBe(pluginScreenRoute(guide));
    expect(screen.getByText('<b>Plain text</b>').querySelector('b')).toBeNull();
    expect(screen.getAllByRole('listitem')).toHaveLength(2);
    fireEvent.click(screen.getByRole('button', { name: 'Back to gallery' }));
    expect(screen.getByLabelText('Active view').textContent).toBe('gallery');
    await openGuide();
    expect(load).toHaveBeenCalledTimes(1);

    invalidate();
    expect(screen.queryByRole('heading', { name: 'Review guide' })).toBeNull();
    expect(screen.getByLabelText('Active view').textContent).toBe(pluginScreenRoute(guide));
    resolveCurrent({ sections: [], screens: [otherGuide], commands: [] });
    await waitFor(() => expect(screen.getByLabelText('Active view').textContent).toBe('gallery'));

    invalidate();
    await openGuide();
    setCatalog('catalog-b');
    expect(screen.queryByRole('heading', { name: 'Review guide' })).toBeNull();
    await waitFor(() => expect(screen.getByLabelText('Active view').textContent).toBe('gallery'));
    expect(screen.queryByRole('button', { name: 'Plugin screens' })).toBeNull();
});
