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

    it('clears an active advanced search when a quick filter is selected', () => {
        filterActions.setAdvancedSearch({
            id: 'advanced',
            logicalOperator: 'and',
            items: []
        });

        filterActions.toggleFavorites();

        expect(filterState.advancedSearch).toBeNull();
        expect(filterState.filterFavorites).toBe(true);
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

    it('clears tags, folders and quick filters when advanced search is selected', () => {
        filterActions.selectTag('tag');
        filterActions.selectFolder('folder');
        filterActions.toggleTrash();

        filterActions.setAdvancedSearch({
            id: 'advanced',
            logicalOperator: 'and',
            items: []
        });

        expect(filterState.selectedTags).toEqual([]);
        expect(filterState.selectedFolderIds).toEqual([]);
        expect(filterState.filterTrash).toBe(false);
        expect(filterState.advancedSearch).not.toBeNull();
    });

    it('clears advanced search when a tag or folder is selected', () => {
        filterActions.setAdvancedSearch({
            id: 'advanced',
            logicalOperator: 'and',
            items: []
        });

        filterActions.selectTag('tag');
        expect(filterState.advancedSearch).toBeNull();

        filterActions.setAdvancedSearch({
            id: 'advanced',
            logicalOperator: 'and',
            items: []
        });
        filterActions.selectFolder('folder');
        expect(filterState.advancedSearch).toBeNull();
    });
});
