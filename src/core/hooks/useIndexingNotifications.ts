import { createEffect, type Accessor } from 'solid-js';
import { useNotification } from './useNotification';

/** Reports scan outcomes received by the system store. */
export function useIndexingNotifications(outcome: Accessor<'completed' | 'failed' | null>) {
    const notification = useNotification();
    createEffect(() => {
        const current = outcome();
        if (current === 'completed') {
            notification.success('Indexing Complete', 'Library update finished');
        } else if (current === 'failed') {
            notification.error(
                'Indexing Failed',
                'The library update could not be completed. Try restarting the scan.'
            );
        }
    });
}
