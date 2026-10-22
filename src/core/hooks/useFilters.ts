import { filterState, filterActions } from '../store/filter';
import { libraryActions } from '../store/library';

/**
 * Hook providing access to filter state and actions for the application content.
 *
 * @returns {Object} Accessors and methods for filtering and sorting items.
 */
export const useFilters = () => {
    const withRefresh =
        <Args extends unknown[], R>(action: (...args: Args) => R) =>
        (...args: Args) => {
            const result = action(...args);
            libraryActions.refreshAssets(true);
            return result;
        };

    return {
        get selectedTags() {
            return filterState.selectedTags;
        },
        get selectedFolderIds() {
            return filterState.selectedFolderIds;
        },
        get folderRecursiveView() {
            return filterState.folderRecursiveView;
        },
        get filterUntagged() {
            return filterState.filterUntagged;
        },
        get filterHasTags() {
            return filterState.filterHasTags;
        },
        get filterFavorites() {
            return filterState.filterFavorites;
        },
        get filterTrash() {
            return filterState.filterTrash;
        },
        /** Current text search query */
        get searchQuery() {
            return filterState.searchQuery;
        },
        /** Whether the text search uses fuzzy matching */
        get searchFuzzy() {
            return filterState.searchFuzzy;
        },
        get sortBy() {
            return filterState.sortBy;
        },
        get sortOrder() {
            return filterState.sortOrder;
        },
        get layout() {
            return filterState.layout;
        },
        get thumbSize() {
            return filterState.thumbSize;
        },
        get advancedSearch() {
            return filterState.advancedSearch;
        },

        get canGoBack() {
            return filterActions.canGoBack();
        },
        get canGoForward() {
            return filterActions.canGoForward();
        },

        selectTag: withRefresh(filterActions.selectTag),
        toggleTag: withRefresh(filterActions.toggleTag),
        setUntagged: withRefresh(filterActions.setUntagged),
        toggleUntagged: withRefresh(filterActions.toggleUntagged),
        setHasTags: withRefresh(filterActions.setHasTags),
        toggleHasTags: withRefresh(filterActions.toggleHasTags),
        setFavorites: withRefresh(filterActions.setFavorites),
        toggleFavorites: withRefresh(filterActions.toggleFavorites),
        setTrash: withRefresh(filterActions.setTrash),
        toggleTrash: withRefresh(filterActions.toggleTrash),
        selectFolder: withRefresh(filterActions.selectFolder),
        clearFolders: withRefresh(filterActions.clearFolders),
        setFolderRecursiveView: withRefresh(filterActions.setFolderRecursiveView),
        /** Updates the text search query */
        setSearch: withRefresh(filterActions.setSearch),
        /** Toggles fuzzy matching for the text search */
        setSearchFuzzy: withRefresh(filterActions.setSearchFuzzy),
        setSortBy: withRefresh(filterActions.setSortBy),
        setSortOrder: withRefresh(filterActions.setSortOrder),
        setLayout: filterActions.setLayout,
        setThumbSize: filterActions.setThumbSize,
        setAdvancedSearch: withRefresh(filterActions.setAdvancedSearch),
        clearAll: withRefresh(filterActions.clearAll),

        goBack: withRefresh(filterActions.goBack),
        goForward: withRefresh(filterActions.goForward),

        hasActiveFilters: filterActions.hasActiveFilters
    };
};
