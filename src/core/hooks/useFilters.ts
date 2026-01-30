import { filterState, filterActions } from "../store/filterStore";
import { libraryActions } from "../store/libraryStore";

export const useFilters = () => {
  const withRefresh = (action: (...args: any[]) => void) => (...args: any[]) => {
    action(...args);
    libraryActions.refreshImages(true);
  };

  return {
    // State (Read-only proxies)
    get selectedTags() { return filterState.selectedTags; },
    get selectedFolderId() { return filterState.selectedFolderId; },
    get folderRecursiveView() { return filterState.folderRecursiveView; },
    get filterUntagged() { return filterState.filterUntagged; },
    get searchQuery() { return filterState.searchQuery; },
    get sortBy() { return filterState.sortBy; },
    get sortOrder() { return filterState.sortOrder; },
    get layout() { return filterState.layout; },
    get thumbSize() { return filterState.thumbSize; },
    get advancedSearch() { return filterState.advancedSearch; },
    
    // Actions
    toggleTag: withRefresh(filterActions.toggleTag),
    setUntagged: withRefresh(filterActions.setUntagged),
    toggleUntagged: withRefresh(filterActions.toggleUntagged),
    setFolder: withRefresh(filterActions.setFolder),
    setFolderRecursiveView: withRefresh(filterActions.setFolderRecursiveView),
    setSearch: withRefresh(filterActions.setSearch),
    setSortBy: withRefresh(filterActions.setSortBy),
    setSortOrder: withRefresh(filterActions.setSortOrder),
    setLayout: withRefresh(filterActions.setLayout),
    setThumbSize: withRefresh(filterActions.setThumbSize),
    setAdvancedSearch: withRefresh(filterActions.setAdvancedSearch),
    clearAll: withRefresh(filterActions.clearAll),
    hasActiveFilters: filterActions.hasActiveFilters
  };
};

