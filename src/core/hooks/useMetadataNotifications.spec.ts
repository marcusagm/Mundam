import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { BatchChangePayload } from '../store/library/schemas';

const notifications = vi.hoisted(() => ({ success: vi.fn(), info: vi.fn() }));
const listeners = vi.hoisted(() => [] as ((event: { payload: BatchChangePayload }) => void)[]);

vi.mock('solid-js', () => ({
    onMount: (callback: () => void) => callback(),
    onCleanup: vi.fn()
}));
vi.mock('./useNotification', () => ({ useNotification: () => notifications }));
vi.mock('@tauri-apps/api/event', () => ({
    listen: async (_event: string, listener: (event: { payload: BatchChangePayload }) => void) => {
        listeners.push(listener);
        return vi.fn();
    }
}));

import { useMetadataNotifications } from './useMetadataNotifications';

describe('folder removal feedback', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        listeners.length = 0;
        useMetadataNotifications();
    });

    it('leaves folder-removal feedback to the confirmation modal', () => {
        listeners[0]({
            payload: {
                operation: 'folder_removal',
                needs_refresh: true,
                removed: [{ id: 'asset', folder_id: 'folder', tag_ids: [] }]
            }
        });

        expect(notifications.info).not.toHaveBeenCalled();
        expect(notifications.success).not.toHaveBeenCalled();
    });

    it('still reports an ordinary background asset deletion', () => {
        listeners[0]({
            payload: { removed: [{ id: 'asset', folder_id: 'folder', tag_ids: [] }] }
        });

        expect(notifications.info).toHaveBeenCalledExactlyOnceWith(
            'Library Sync',
            '1 asset removed'
        );
    });
});
