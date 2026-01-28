import { createStore } from "solid-js/store";


interface FilterState {
  selectedTags: number[];
  selectedLocationId: number | null;
  selectedSubfolderId: number | null;
  filterUntagged: boolean;
  searchQuery: string;
}

const [filterState, setFilterState] = createStore<FilterState>({
  selectedTags: [],
  selectedLocationId: null,
  selectedSubfolderId: null,
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

  setLocation: (locationId: number | null) => {
    setFilterState({
      selectedLocationId: locationId,
      selectedSubfolderId: null
    });
  },

  setSubfolder: (subfolderId: number | null) => {
    setFilterState("selectedSubfolderId", subfolderId);
  },

  // Atomic setter to avoid race conditions
  setFolder: (locationId: number | null, subfolderId: number | null) => {
    setFilterState({
      selectedLocationId: locationId,
      selectedSubfolderId: subfolderId
    });
  },

  setSearch: (query: string) => {
    setFilterState("searchQuery", query);
  },

  clearAll: () => {
    setFilterState({
      selectedTags: [],
      selectedLocationId: null,
      selectedSubfolderId: null,
      filterUntagged: false,
      searchQuery: ""
    });
  },

  hasActiveFilters: () => {
    return filterState.selectedTags.length > 0 || 
           filterState.filterUntagged || 
           filterState.selectedLocationId !== null ||
           filterState.selectedSubfolderId !== null;
  }
};

export { filterState };

