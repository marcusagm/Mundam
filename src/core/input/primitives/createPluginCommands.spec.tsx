import { cleanup, fireEvent, render, screen, within } from '@solidjs/testing-library';
import { createSignal, Show, type JSX } from 'solid-js';
import { afterEach, expect, it, vi } from 'vitest';
import { KeyboardShortcutsPanel } from '../../../components/features/settings/KeyboardShortcutsPanel';
import { PluginCommandsMenu } from '../../../components/layout/TitleBar/PluginCommandsMenu';
import { invokeCommand } from '../../../lib/api';
import {
    pluginCommandKey,
    type PluginCommand,
    type PluginPresentation
} from '../../../lib/pluginPresentation';
import { InputProvider } from '../context';
import { shortcutStore } from '../store/shortcutStore';
import { createPluginCommands } from './createPluginCommands';

vi.mock('../../../lib/api', () => ({ invokeCommand: vi.fn().mockResolvedValue(null) }));
vi.mock('../../../components/ui/SectionGroup', () => ({
    SectionGroup: (props: { title: string; children: JSX.Element }) => (
        <Show when={props.title === 'Plugin commands'}>
            <section>{props.children}</section>
        </Show>
    )
}));
vi.mock('../../../components/features/settings/MouseInteractionsReference', () => ({
    MouseInteractionsReference: () => null
}));

const command: PluginCommand = {
    pluginId: 'org.example.command-test',
    pluginName: 'Command check',
    commandId: 'open-guide',
    title: 'Open guide',
    action: { kind: 'openScreen', screen: 'guide' },
    suggestedShortcut: 'Mod+Shift+KeyJ'
};
const approved: PluginPresentation = {
    sections: [],
    screens: [
        {
            pluginId: command.pluginId,
            pluginName: command.pluginName,
            screenId: 'guide',
            title: 'Guide',
            blocks: [{ kind: 'paragraph', text: 'Read the guide.' }]
        }
    ],
    commands: [command]
};

afterEach(cleanup);

it('keeps commands callable by menu, requires shortcut opt-in and respects conflicts and revocation', async () => {
    const [presentation, setPresentation] = createSignal<PluginPresentation | undefined>(approved);
    const navigate = vi.fn();
    const key = pluginCommandKey(command);
    const { unmount } = render(() => {
        const commands = createPluginCommands(presentation, navigate);
        return (
            <InputProvider>
                <PluginCommandsMenu commands={commands.available()} onRun={commands.run} />
                <KeyboardShortcutsPanel />
            </InputProvider>
        );
    });
    const row = () =>
        within(
            screen
                .getByText('Open guide · Command check', { selector: '.shortcut-name' })
                .closest<HTMLElement>('.shortcut-row')!
        );
    const pressSuggestion = () => {
        const event = { key: 'J', code: 'KeyJ', ctrlKey: true, shiftKey: true };
        fireEvent.keyDown(document.body, event);
        fireEvent.keyUp(document.body, event);
    };
    expect(row().getByText('Unassigned')).toBeTruthy();
    pressSuggestion();
    expect(navigate).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: 'Plugin commands' }));
    fireEvent.click(await screen.findByRole('menuitem', { name: 'Open guide · Command check' }));
    expect(navigate).toHaveBeenCalledExactlyOnceWith('plugin:org.example.command-test:guide');

    const blocker = shortcutStore.register({
        name: 'Existing binding',
        keys: 'Mod+Shift+KeyJ',
        scope: 'global'
    });
    try {
        fireEvent.click(row().getByRole('button', { name: /Use suggested/ }));
        expect(row().getByText(/Conflicts with: Existing binding/)).toBeTruthy();
        expect(shortcutStore.getById(key)?.keys).toBe('');
        fireEvent.click(row().getByTitle('Cancel'));
    } finally {
        shortcutStore.unregister(blocker);
    }

    fireEvent.click(row().getByRole('button', { name: /Use suggested/ }));
    expect(invokeCommand).toHaveBeenCalledWith(
        'set_setting',
        expect.objectContaining({
            value: expect.objectContaining({ [`id:${key}`]: 'Mod+Shift+KeyJ' })
        })
    );
    pressSuggestion();
    expect(navigate).toHaveBeenCalledTimes(2);
    const returnedBlocker = shortcutStore.register({
        name: 'Existing binding',
        keys: 'Mod+Shift+KeyJ',
        scope: 'global'
    });
    try {
        expect(row().getByText(/Shortcut inactive/)).toBeTruthy();
        pressSuggestion();
        expect(navigate).toHaveBeenCalledTimes(2);
    } finally {
        shortcutStore.unregister(returnedBlocker);
    }
    setPresentation(undefined);
    expect(screen.queryByRole('button', { name: 'Plugin commands' })).toBeNull();
    expect(shortcutStore.getById(key)).toBeUndefined();
    pressSuggestion();
    expect(navigate).toHaveBeenCalledTimes(2);

    setPresentation({ ...approved, commands: [{ ...command, title: 'Renamed guide' }] });
    expect(shortcutStore.getById(key)?.keys).toBeTruthy();
    pressSuggestion();
    expect(navigate).toHaveBeenCalledTimes(3);
    shortcutStore.resetToDefault(key);
    unmount();
});
