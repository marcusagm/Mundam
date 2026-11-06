import { describe, expect, it } from 'vitest';
import { planLibraryBatchUpdates, type LibraryBatchView } from './libraryBatchUpdates';

const view: LibraryBatchView = {
    selectedFolderIds: ['root'],
    recursive: true,
    locations: [
        { id: 'root', parent_id: null },
        { id: 'child', parent_id: 'root' },
        { id: 'outside', parent_id: null }
    ],
    currentItems: [{ id: 'known' }, { id: 'leaving' }]
};

function update(id: string, folderId: string) {
    return {
        id,
        folder_id: folderId,
        path: `/${folderId}/${id}.png`,
        filename: `${id}.png`,
        modified_at: '2026-11-06T12:00:00Z'
    };
}

describe('library batch folder membership', () => {
    it('patches known descendants, removes moved-out assets and refreshes for incoming assets', () => {
        const known = update('known', 'child');
        const plan = planLibraryBatchUpdates(
            [known, update('leaving', 'outside'), update('incoming', 'child')],
            view
        );
        expect(plan.updates).toEqual([known]);
        expect([...plan.removedIds]).toEqual(['leaving']);
        expect(plan.needsRefresh).toBe(true);
        expect(view.currentItems).toEqual([{ id: 'known' }, { id: 'leaving' }]);
    });

    it('keeps direct-folder selection distinct from recursive selection', () => {
        const plan = planLibraryBatchUpdates([update('known', 'child')], {
            ...view,
            recursive: false
        });
        expect(plan.updates).toEqual([]);
        expect([...plan.removedIds]).toEqual(['known']);
        expect(plan.needsRefresh).toBe(false);
    });

    it('allows unfiltered folders and terminates on cyclic parent metadata', () => {
        expect(
            planLibraryBatchUpdates([update('known', 'outside')], {
                ...view,
                selectedFolderIds: []
            }).updates
        ).toHaveLength(1);
        const plan = planLibraryBatchUpdates([update('known', 'cycle')], {
            ...view,
            locations: [{ id: 'cycle', parent_id: 'cycle' }]
        });
        expect([...plan.removedIds]).toEqual(['known']);
    });
});
