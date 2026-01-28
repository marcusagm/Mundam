import { createStore } from "solid-js/store";


interface FilterState {
  selectedTags: number[];
  selectedFolderId: number | null;
  folderRecursiveView: boolean;
  filterUntagged: boolean;
  searchQuery: string;
}

const [filterState, setFilterState] = createStore<FilterState>({
  selectedTags: [],
  selectedFolderId: null,
  folderRecursiveView: false,
  filterUntagged: false,
  searchQuery: ""
});

export const filterActions = {
  toggleTag: (tagId: number) => {
    const current = filterState.selectedTags;
    if (current.includes(tagId)) {
      setFilterState("selectedTags", (tags) => tags.filter(id => id !== tagId));
    } else {
      setFilterState({
        selectedTags: [...current, tagId],
        filterUntagged: false 
      });
    }
  },

  setUntagged: (isActive: boolean) => {
    setFilterState("filterUntagged", isActive);
    if (isActive) {
      setFilterState("selectedTags", []);
    }
  },

  toggleUntagged: () => {
    filterActions.setUntagged(!filterState.filterUntagged);
  },

  setFolder: (folderId: number | null) => {
    setFilterState("selectedFolderId", folderId);
  },

  setFolderRecursiveView: (isRecursive: boolean) => {
    setFilterState("folderRecursiveView", isRecursive);
  },

  setSearch: (query: string) => {
    setFilterState("searchQuery", query);
  },

  clearAll: () => {
    setFilterState({
      selectedTags: [],
      selectedFolderId: null,
      filterUntagged: false,
      searchQuery: ""
    });
  },

  hasActiveFilters: () => {
    return filterState.selectedTags.length > 0 || 
           filterState.filterUntagged || 
           filterState.selectedFolderId !== null;
  }
};

export { filterState };
