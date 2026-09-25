import { batch } from 'solid-js';
import { filterState, filterStateInternal, persist, type FilterSnapshot } from './filterState';

const { setFilterState } = filterStateInternal;

export const historyActions = {
    pushHistory: () => {
        const snapshot: FilterSnapshot = {
            selectedTags: filterState.selectedTags,
            selectedFolderId: filterState.selectedFolderId,
            folderRecursiveView: filterState.folderRecursiveView,
            filterUntagged: filterState.filterUntagged,
            searchQuery: filterState.searchQuery,
            searchFuzzy: filterState.searchFuzzy,
            advancedSearch: filterState.advancedSearch,
            sortBy: filterState.sortBy,
            sortOrder: filterState.sortOrder,
            filterHasTags: filterState.filterHasTags,
            filterFavorites: filterState.filterFavorites,
            filterTrash: filterState.filterTrash
        };

        // Check if the current state is different from the last history item
        const current = filterState.history[filterState.historyIndex];
        const isSame = JSON.stringify(snapshot) === JSON.stringify(current);

        if (isSame) return;

        const newHistory = filterState.history.slice(0, filterState.historyIndex + 1);
        newHistory.push(snapshot);

        // Limit history
        const limit = filterState.historyLimit || 50;
        const finalHistory =
            newHistory.length > limit ? newHistory.slice(newHistory.length - limit) : newHistory;

        setFilterState({
            history: finalHistory,
            historyIndex: finalHistory.length - 1
        });
    },

    setHistoryLimit: (limit: number) => {
        setFilterState('historyLimit', limit);
        persist({ historyLimit: limit });
    },

    goBack: () => {
        if (filterState.historyIndex > 0) {
            const prevIndex = filterState.historyIndex - 1;
            const snapshot = filterState.history[prevIndex];
            batch(() => {
                setFilterState({
                    ...snapshot,
                    historyIndex: prevIndex
                });
            });
        }
    },

    goForward: () => {
        if (filterState.historyIndex < filterState.history.length - 1) {
            const nextIndex = filterState.historyIndex + 1;
            const snapshot = filterState.history[nextIndex];
            batch(() => {
                setFilterState({
                    ...snapshot,
                    historyIndex: nextIndex
                });
            });
        }
    },

    canGoBack: () => filterState.historyIndex > 0,
    canGoForward: () => filterState.historyIndex < filterState.history.length - 1
};
