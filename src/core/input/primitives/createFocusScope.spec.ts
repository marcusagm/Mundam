import { createRoot } from 'solid-js';
import { expect, it, vi } from 'vitest';
import { createFocusScope } from './createFocusScope';

vi.mock('./createInputScope', () => ({ createConditionalScope: vi.fn() }));

it('checks current focus when a retained command handler runs and preserves descendant focus', () => {
    createRoot(dispose => {
        const scope = createFocusScope('video-player');
        const command = vi.fn();
        const guarded = scope.whenFocused(command);
        guarded('inactive');
        expect(command).not.toHaveBeenCalled();
        scope.onFocusIn();
        guarded('focused');
        const container = document.createElement('div');
        const descendant = document.createElement('button');
        container.append(descendant);
        container.addEventListener('focusout', event => {
            scope.onFocusOut(event as FocusEvent & { currentTarget: HTMLElement });
        });
        container.dispatchEvent(new FocusEvent('focusout', { relatedTarget: descendant }));
        guarded('descendant');
        container.dispatchEvent(new FocusEvent('focusout', { relatedTarget: null }));
        guarded('blurred');
        expect(command.mock.calls).toEqual([['focused'], ['descendant']]);
        dispose();
    });
});
