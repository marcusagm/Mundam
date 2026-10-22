import { beforeEach, describe, expect, it } from 'vitest';
import { filterActions } from './filterActions';
import { defaultSnapshot, filterState, filterStateInternal } from './filterState';

describe('quick filter selection', () => {
    beforeEach(() => {
        filterStateInternal.setFilterState({
            ...defaultSnapshot,
            history: [{ ...defaultSnapshot }],
            historyIndex: 0
        });
    });

    it('replaces the active quick filter on an ordinary selection', () => {
        filterActions.toggleFavorites();
        filterActions.toggleTrash();

        expect(filterState.filterFavorites).toBe(false);
        expect(filterState.filterTrash).toBe(true);
    });

    it('never combines Untagged with Has Tags', () => {
        filterActions.toggleUntagged();
        filterActions.toggleHasTags();

        expect(filterState.filterUntagged).toBe(false);
        expect(filterState.filterHasTags).toBe(true);
    });

    it('deactivates a quick filter selected twice', () => {
        filterActions.toggleTrash();
        filterActions.toggleTrash();

        expect(filterState.filterTrash).toBe(false);
    });
});

describe('tag and folder selection', () => {
    beforeEach(() => {
        filterStateInternal.setFilterState({
            ...defaultSnapshot,
            history: [{ ...defaultSnapshot }],
            historyIndex: 0
        });
    });

    it('replaces the selected tag on an ordinary click', () => {
        filterActions.selectTag('a');
        filterActions.selectTag('b');

        expect(filterState.selectedTags).toEqual(['b']);
    });

    it('accumulates and removes tags with additive selection', () => {
        filterActions.selectTag('a');
        filterActions.selectTag('b', true);
        expect(filterState.selectedTags).toEqual(['a', 'b']);

        filterActions.selectTag('a', true);
        expect(filterState.selectedTags).toEqual(['b']);
    });

    it('clears the only selected tag when clicked again', () => {
        filterActions.selectTag('a');
        filterActions.selectTag('a');

        expect(filterState.selectedTags).toEqual([]);
    });

    it('replaces and accumulates folders the same way', () => {
        filterActions.selectFolder('f1');
        filterActions.selectFolder('f2');
        expect(filterState.selectedFolderIds).toEqual(['f2']);

        filterActions.selectFolder('f1', true);
        expect(filterState.selectedFolderIds).toEqual(['f2', 'f1']);
    });
});
