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

    it('adds a quick filter when selected additively', () => {
        filterActions.toggleFavorites();
        filterActions.toggleTrash(true);

        expect(filterState.filterFavorites).toBe(true);
        expect(filterState.filterTrash).toBe(true);
    });
});
