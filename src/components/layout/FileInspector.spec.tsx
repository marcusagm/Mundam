import { cleanup, fireEvent, render, screen, waitFor } from '@solidjs/testing-library';
import { createSignal } from 'solid-js';
import { afterEach, expect, it, vi } from 'vitest';
import { InputProvider } from '../../core/input';
import type { PluginInspectorClient, PluginInspectorSection } from '../../lib/pluginInspector';
import { createPluginInspectorPresentation } from '../features/inspector/plugins/createPluginInspectorPresentation';
import { PluginViewportActions } from '../features/viewport/toolbar/PluginViewportActions';
import { FileInspector } from './FileInspector';

const selection = vi.hoisted(() => ({ read: (): string[] => [] }));

vi.mock('../../lib/api', () => ({ invokeCommand: vi.fn().mockResolvedValue(null) }));
vi.mock('../../core/hooks', () => ({
    useLibrary: () => ({
        items: [
            { id: 'one', filename: 'one.png' },
            { id: 'two', filename: 'two.png' }
        ]
    }),
    useSelection: () => ({
        get selectedIds() {
            return selection.read();
        }
    })
}));
vi.mock('../features/inspector/image/ImageInspector', () => ({
    ImageInspector: () => <div>Native image details</div>
}));
vi.mock('../features/inspector/audio/AudioInspector', () => ({ AudioInspector: () => null }));
vi.mock('../features/inspector/video/VideoInspector', () => ({ VideoInspector: () => null }));
vi.mock('../features/inspector/font/FontInspector', () => ({ FontInspector: () => null }));
vi.mock('../features/inspector/model/ModelInspector', () => ({ ModelInspector: () => null }));
vi.mock('../features/inspector/multi/MultiInspector', () => ({
    MultiInspector: () => <div>Native multi-selection</div>
}));

const reviewSection: PluginInspectorSection = {
    pluginId: 'org.example.reference-review',
    pluginName: 'Reference review',
    sectionId: 'review',
    title: 'Review',
    viewportActionLabel: 'Show review in Inspector',
    fields: [
        { fieldId: 'reviewed', label: 'Reference reviewed', kind: 'checkbox', defaultValue: false }
    ]
};

function renderInspector(client: PluginInspectorClient) {
    const [selected, setSelected] = createSignal<string[]>([]);
    const [open, setOpen] = createSignal(true);
    const [reveal, setReveal] = createSignal<string>();
    selection.read = selected;
    render(() => {
        const presentation = createPluginInspectorPresentation(
            () => client,
            () => selected().length === 1
        );
        return (
            <InputProvider>
                <PluginViewportActions
                    sections={presentation.sections()}
                    onRevealSection={key => {
                        setOpen(true);
                        setReveal(key);
                    }}
                />
                <FileInspector
                    pluginInspector={presentation}
                    inspectorOpen={open()}
                    revealSection={reveal()}
                    onSectionRevealed={() => setReveal(undefined)}
                />
            </InputProvider>
        );
    });
    return { setSelected, setOpen };
}

afterEach(cleanup);

it('composes signed plugin definitions only for one visible selection', async () => {
    const unsubscribe = vi.fn();
    const load = vi.fn().mockResolvedValue([reviewSection]);
    const { setSelected, setOpen } = renderInspector({ load, subscribe: () => unsubscribe });
    expect(screen.queryByRole('checkbox')).toBeNull();
    expect(load).not.toHaveBeenCalled();
    setSelected(['one']);
    const checkbox = await screen.findByRole<HTMLButtonElement>('checkbox', {
        name: 'Reference reviewed'
    });
    expect(checkbox.disabled).toBe(true);
    expect(checkbox.getAttribute('aria-checked')).toBe('false');
    expect(screen.getByText('Native image details')).toBeTruthy();
    setSelected(['two']);
    expect(screen.getByRole('checkbox', { name: 'Reference reviewed' })).toBeTruthy();
    expect(load).toHaveBeenCalledTimes(1);
    setSelected(['one', 'two']);
    expect(screen.getByText('Native multi-selection')).toBeTruthy();
    expect(screen.queryByRole('checkbox')).toBeNull();
    expect(unsubscribe).toHaveBeenCalledTimes(1);
    setOpen(false);
    setSelected(['one']);
    await screen.findByRole('button', { name: 'Plugin actions' });
    expect(screen.queryByRole('checkbox')).toBeNull();
    setOpen(true);
    await screen.findByRole('checkbox', { name: 'Reference reviewed' });
    expect(load).toHaveBeenCalledTimes(2);
});

it('refreshes approval changes and ignores results that predate invalidation', async () => {
    let refresh = () => {};
    let resolveStale!: (value: PluginInspectorSection[]) => void;
    const stale = new Promise<PluginInspectorSection[]>(resolve => {
        resolveStale = resolve;
    });
    const load = vi
        .fn()
        .mockReturnValueOnce(stale)
        .mockResolvedValueOnce([])
        .mockResolvedValueOnce([reviewSection])
        .mockResolvedValueOnce([]);
    const { setSelected } = renderInspector({
        load,
        subscribe: onChange => {
            refresh = onChange;
            return vi.fn();
        }
    });
    setSelected(['one']);
    await waitFor(() => expect(load).toHaveBeenCalledTimes(1));
    refresh();
    await waitFor(() => expect(load).toHaveBeenCalledTimes(2));
    resolveStale([reviewSection]);
    await stale;
    expect(screen.queryByRole('checkbox')).toBeNull();
    refresh();
    await screen.findByRole('checkbox', { name: 'Reference reviewed' });
    refresh();
    await waitFor(() => expect(screen.queryByRole('checkbox')).toBeNull());
    expect(screen.queryByRole('button', { name: 'Plugin actions' })).toBeNull();
    expect(load).toHaveBeenCalledTimes(4);
});

it('shares one query with the toolbar and focuses the supplying plugin section', async () => {
    const anotherSection = {
        ...reviewSection,
        pluginId: 'org.other.review',
        pluginName: 'Other',
        viewportActionLabel: undefined
    };
    const load = vi.fn().mockResolvedValue([anotherSection, reviewSection]);
    const { setOpen, setSelected } = renderInspector({ load, subscribe: () => vi.fn() });
    setOpen(false);
    setSelected(['one']);

    const invokeAction = async () => {
        fireEvent.click(await screen.findByRole('button', { name: 'Plugin actions' }));
        fireEvent.click(
            await screen.findByRole('menuitem', {
                name: 'Show review in Inspector · Reference review'
            })
        );
    };
    await invokeAction();
    const [other, target] = await screen.findAllByRole('button', { name: 'Review' });
    expect(document.activeElement).toBe(target);
    fireEvent.click(target);
    expect(target.getAttribute('aria-expanded')).toBe('false');
    await invokeAction();
    expect(target.getAttribute('aria-expanded')).toBe('true');
    expect(other.getAttribute('aria-expanded')).toBe('true');
    expect(document.activeElement).toBe(target);
    expect(load).toHaveBeenCalledTimes(1);

    setSelected(['one', 'two']);
    expect(screen.queryByRole('button', { name: 'Plugin actions' })).toBeNull();
});
