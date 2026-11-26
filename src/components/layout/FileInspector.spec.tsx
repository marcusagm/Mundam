import { cleanup, fireEvent, render, screen, waitFor } from '@solidjs/testing-library';
import { createSignal } from 'solid-js';
import { afterEach, expect, it, vi } from 'vitest';
import { InputProvider } from '../../core/input';
import type { PluginInspectorDataClient, PluginInspectorSection } from '../../lib/pluginInspector';
import { createPluginPresentation } from '../../core/hooks/createPluginPresentation';
import type { PluginPresentationClient } from '../../lib/pluginPresentation';
import { PluginSectionActionMenu } from '../features/inspector/plugins/PluginSectionActionMenu';
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
    packageRevision: 'ab'.repeat(32),
    title: 'Review',
    viewportActionLabel: 'Show review in Inspector',
    fields: [
        { fieldId: 'reviewed', label: 'Reference reviewed', kind: 'checkbox', defaultValue: false }
    ]
};

const readOnlyData: PluginInspectorDataClient = {
    load: async () => [
        {
            fieldId: 'reviewed',
            state: { status: 'available', value: false, revision: '0', writable: false }
        }
    ],
    save: vi.fn()
};

function renderInspector(
    client: {
        load: () => Promise<PluginInspectorSection[]>;
        subscribe: PluginPresentationClient['subscribe'];
    },
    data: PluginInspectorDataClient = readOnlyData
) {
    const [selected, setSelected] = createSignal<string[]>([]);
    const [open, setOpen] = createSignal(true);
    const [reveal, setReveal] = createSignal<string>();
    selection.read = selected;
    render(() => {
        const presentation = createPluginPresentation(
            () => ({
                load: async () => ({ sections: await client.load(), screens: [], commands: [] }),
                subscribe: client.subscribe
            }),
            () => (selected().length === 1 ? 'test-catalog' : undefined)
        );
        const revealSection = (key: string) => {
            setOpen(true);
            setReveal(key);
        };
        return (
            <InputProvider>
                <PluginSectionActionMenu
                    surface="viewport"
                    buttonLabel="Plugin actions"
                    sections={presentation.sections()}
                    onRevealSection={revealSection}
                />
                <PluginSectionActionMenu
                    surface="titleBar"
                    buttonLabel="Title bar plugin actions"
                    sections={presentation.sections()}
                    onRevealSection={revealSection}
                />
                <FileInspector
                    pluginInspector={presentation}
                    pluginInspectorData={data}
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

it('uses confirmed revisions and leaves a new selection unchanged by an older save', async () => {
    const state = (value: boolean, revision: string) => [
        {
            fieldId: 'reviewed',
            state: { status: 'available' as const, value, revision, writable: true }
        }
    ];
    const load = vi.fn<PluginInspectorDataClient['load']>().mockResolvedValue(state(false, '0'));
    let resolveFirst!: (value: Awaited<ReturnType<PluginInspectorDataClient['save']>>) => void;
    const pending = new Promise<Awaited<ReturnType<PluginInspectorDataClient['save']>>>(resolve => {
        resolveFirst = resolve;
    });
    const save = vi
        .fn<PluginInspectorDataClient['save']>()
        .mockReturnValueOnce(pending)
        .mockResolvedValueOnce({ status: 'conflict' });
    const { setSelected } = renderInspector(
        {
            load: async () => [reviewSection],
            subscribe: () => vi.fn()
        },
        { load, save }
    );
    const checkbox = () =>
        screen.findByRole<HTMLButtonElement>('checkbox', { name: 'Reference reviewed' });

    setSelected(['one']);
    fireEvent.click(await checkbox());
    expect(save).toHaveBeenNthCalledWith(1, {
        target: {
            assetId: 'one',
            pluginId: reviewSection.pluginId,
            sectionId: 'review',
            packageRevision: reviewSection.packageRevision
        },
        fieldId: 'reviewed',
        expectedRevision: '0',
        value: true
    });
    setSelected(['two']);
    await checkbox();
    resolveFirst({ status: 'saved', value: true, revision: '1' });
    await pending;
    expect((await checkbox()).getAttribute('aria-checked')).toBe('false');

    load.mockResolvedValueOnce(state(true, '4'));
    fireEvent.click(await checkbox());
    expect(await screen.findByText(/This value changed elsewhere/)).toBeTruthy();
    await waitFor(async () => expect((await checkbox()).getAttribute('aria-checked')).toBe('true'));
    expect(save).toHaveBeenCalledTimes(2);
});

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
    expect(await screen.findByRole('checkbox', { name: 'Reference reviewed' })).toBeTruthy();
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

it('opens an opted-in title bar action with the menu keyboard shortcuts', async () => {
    const titleAction = {
        ...reviewSection,
        sectionId: 'title-review',
        title: 'Title bar review',
        titleBarActionLabel: 'Open title review',
        viewportActionLabel: undefined
    };
    const load = vi.fn().mockResolvedValue([reviewSection, titleAction]);
    const { setOpen, setSelected } = renderInspector({ load, subscribe: () => vi.fn() });
    setOpen(false);
    setSelected(['one']);
    const trigger = await screen.findByRole('button', { name: 'Title bar plugin actions' });
    expect(trigger.getAttribute('aria-expanded')).toBe('false');
    trigger.focus();
    fireEvent.click(trigger);
    expect(trigger.getAttribute('aria-expanded')).toBe('true');
    expect(
        await screen.findByRole('menuitem', { name: 'Open title review · Reference review' })
    ).toBeTruthy();
    expect(screen.getAllByRole('menuitem')).toHaveLength(1);

    fireEvent.keyDown(trigger, { key: 'ArrowDown', code: 'ArrowDown' });
    fireEvent.keyUp(trigger, { key: 'ArrowDown', code: 'ArrowDown' });
    fireEvent.keyDown(trigger, { key: 'Enter', code: 'Enter' });
    fireEvent.keyUp(trigger, { key: 'Enter', code: 'Enter' });
    const section = await screen.findByRole('button', { name: 'Title bar review' });
    expect(document.activeElement).toBe(section);
    expect(trigger.getAttribute('aria-expanded')).toBe('false');
    expect(load).toHaveBeenCalledTimes(1);
});
