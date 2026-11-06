import { cleanup, fireEvent, render, screen } from '@solidjs/testing-library';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ContextMenuItem } from '../../ui/ContextMenu';
import type { JSX } from 'solid-js';
import { FolderContextMenu } from './FolderContextMenu';
import { TagContextMenu } from '../tags/TagContextMenu';

const state = vi.hoisted(() => ({
    item: {
        id: 'child',
        name: 'Child',
        path: '/parent/child',
        parent_id: 'parent',
        color: '#112233' as string | null,
        inherits_color: false
    },
    updateTag: vi.fn(),
    execute: vi.fn(),
    close: vi.fn()
}));
vi.mock('../../../core/hooks', () => ({
    useMetadata: () => ({
        locations: [state.item],
        tags: [state.item],
        updateTag: state.updateTag
    }),
    useFilters: () => ({ folderRecursiveView: false, setFolderRecursiveView: vi.fn() })
}));
vi.mock('../../../core/store/library/folderActions', () => ({
    executeLibraryOperation: state.execute
}));
vi.mock('../../ui/ColorPicker', () => ({
    ColorPicker: () => <div data-testid="shared-color-picker" />
}));
vi.mock('@tauri-apps/plugin-opener', () => ({ revealItemInDir: vi.fn() }));
vi.mock('@tauri-apps/plugin-clipboard-manager', () => ({ writeText: vi.fn() }));
vi.mock('../../ui/ContextMenu', async () => {
    const { ContextMenuCheckboxItem } =
        await import('../../ui/ContextMenu/ContextMenuCheckboxItem');
    const items = (entries: ContextMenuItem[]): JSX.Element =>
        entries.map(item => {
            if (item.type === 'custom') return item.content;
            if (item.type === 'item') return <button onClick={item.action}>{item.label}</button>;
            if (item.type === 'submenu')
                return (
                    <div>
                        {item.label}
                        {items(item.items)}
                    </div>
                );
            return <hr />;
        });
    return {
        ContextMenuCheckboxItem,
        ContextMenu: (props: { items: ContextMenuItem[] }) => <div>{items(props.items)}</div>
    };
});
afterEach(cleanup);
beforeEach(() => {
    vi.clearAllMocks();
    state.item.color = '#112233';
    state.item.inherits_color = false;
    state.execute.mockResolvedValue({ id: 'child' });
});

function showMenu(kind: 'folder' | 'tag'): void {
    if (kind === 'folder')
        render(() => (
            <FolderContextMenu
                coordinateX={0}
                coordinateY={0}
                isOpen
                node={{
                    id: 'folder-child',
                    label: 'Child',
                    data: { folderIdentifier: 'child', path: '/parent/child', isRoot: false }
                }}
                onClose={state.close}
                onDelete={() => undefined}
                onRename={() => undefined}
                onAddChild={() => undefined}
                onAction={() => undefined}
            />
        ));
    else
        render(() => (
            <TagContextMenu
                coordinateX={0}
                coordinateY={0}
                isOpen
                node={{ id: 'child', label: 'Child' }}
                onClose={state.close}
                onDelete={() => undefined}
                onRename={() => undefined}
                onAddChild={() => undefined}
            />
        ));
}

describe.each(['folder', 'tag'] as const)('%s color context menu', kind => {
    it('shows Reset Color only for an active custom color', () => {
        showMenu(kind);
        expect(screen.getByRole('button', { name: 'Reset Color' })).toBeTruthy();
        cleanup();
        state.item.inherits_color = true;
        showMenu(kind);
        expect(screen.queryByRole('button', { name: 'Reset Color' })).toBeNull();
        cleanup();
        state.item.inherits_color = false;
        state.item.color = null;
        showMenu(kind);
        expect(screen.queryByRole('button', { name: 'Reset Color' })).toBeNull();
    });
    it('uses the shared checkbox row and closes after changing inheritance', () => {
        showMenu(kind);
        const checkbox = screen.getByRole('checkbox', { name: 'Use Parent Color' });
        expect(checkbox.closest('.ui-context-menu-item')).toBeTruthy();
        fireEvent.click(checkbox);
        expect(state.close).toHaveBeenCalledOnce();
        if (kind === 'folder')
            expect(state.execute).toHaveBeenCalledWith({
                kind: 'folderColor',
                folderId: 'child',
                color: '#112233',
                inheritsColor: true
            });
        else
            expect(state.updateTag).toHaveBeenCalledWith('child', {
                color: '#112233',
                inheritsColor: true
            });
    });
    it('clears custom color without enabling inheritance', () => {
        showMenu(kind);
        fireEvent.click(screen.getByRole('button', { name: 'Reset Color' }));
        if (kind === 'folder')
            expect(state.execute).toHaveBeenCalledWith({
                kind: 'folderColor',
                folderId: 'child',
                color: null,
                inheritsColor: false
            });
        else
            expect(state.updateTag).toHaveBeenCalledWith('child', {
                inheritsColor: false,
                resetColor: true
            });
    });
});
