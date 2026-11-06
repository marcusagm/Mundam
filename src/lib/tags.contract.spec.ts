import { expect, it, vi } from 'vitest';
import { invokeCommand } from './api';
import { tagService } from './tags';

vi.mock('./api', () => ({ invokeCommand: vi.fn().mockResolvedValue(undefined) }));

it('sends tag changes in the typed IPC payload, preserving false and explicit reset', async () => {
    await tagService.updateTag('tag', {
        name: 'Renamed',
        parentId: 'parent',
        inheritsColor: false,
        resetColor: true
    });
    expect(invokeCommand).toHaveBeenCalledExactlyOnceWith('update_tag', {
        payload: {
            id: 'tag',
            name: 'Renamed',
            parentId: 'parent',
            inheritsColor: false,
            resetColor: true
        }
    });
});
