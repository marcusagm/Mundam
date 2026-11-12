import { describe, expect, it, vi } from 'vitest';

const notifications = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn() }));
vi.mock('solid-js', () => ({ createEffect: (callback: () => void) => callback() }));
vi.mock('./useNotification', () => ({ useNotification: () => notifications }));

import { useIndexingNotifications } from './useIndexingNotifications';

describe('indexing notifications', () => {
    it('reports failure without reporting success', () => {
        vi.clearAllMocks();
        useIndexingNotifications(() => 'failed');
        expect(notifications.error).toHaveBeenCalledOnce();
        expect(notifications.success).not.toHaveBeenCalled();
    });

    it('reports successful completion', () => {
        vi.clearAllMocks();
        useIndexingNotifications(() => 'completed');
        expect(notifications.success).toHaveBeenCalledOnce();
        expect(notifications.error).not.toHaveBeenCalled();
    });
});
