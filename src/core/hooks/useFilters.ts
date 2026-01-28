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
    get selectedLocationId() { return filterState.selectedLocationId; },
    get selectedSubfolderId() { return filterState.selectedSubfolderId; },
    get filterUntagged() { return filterState.filterUntagged; },
    get searchQuery() { return filterState.searchQuery; },
    
    // Actions
    toggleTag: withRefresh(filterActions.toggleTag),
    setUntagged: withRefresh(filterActions.setUntagged),
    toggleUntagged: withRefresh(filterActions.toggleUntagged),
    setLocation: withRefresh(filterActions.setLocation),
    setSubfolder: withRefresh(filterActions.setSubfolder),
    setFolder: withRefresh(filterActions.setFolder),
    setSearch: withRefresh(filterActions.setSearch),
    clearAll: withRefresh(filterActions.clearAll),
    hasActiveFilters: filterActions.hasActiveFilters
  };
};

