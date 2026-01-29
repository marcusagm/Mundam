import { createStore } from "solid-js/store";

export type SortField = "modification" | "addition" | "creation" | "title" | "type" | "size";
export type SortOrder = "asc" | "desc";
export type ViewLayout = "masonry-v" | "masonry-h" | "grid" | "list";

interface FilterState {
  selectedTags: number[];
  selectedFolderId: number | null;
  folderRecursiveView: boolean;
  filterUntagged: boolean;
  searchQuery: string;
  sortBy: SortField;
  sortOrder: SortOrder;
  layout: ViewLayout;
  thumbSize: number;
}

const [filterState, setFilterState] = createStore<FilterState>({
  selectedTags: [],
  selectedFolderId: null,
  folderRecursiveView: false,
  filterUntagged: false,
  searchQuery: "",
  sortBy: "modification",
  sortOrder: "desc",
  layout: "masonry-v",
  thumbSize: 200
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

  setSortBy: (field: SortField) => {
    setFilterState("sortBy", field);
  },

  setSortOrder: (order: SortOrder) => {
    setFilterState("sortOrder", order);
  },

  setLayout: (layout: ViewLayout) => {
    setFilterState("layout", layout);
  },

  setThumbSize: (size: number) => {
    setFilterState("thumbSize", size);
  },

  clearAll: () => {
    setFilterState({
      selectedTags: [],
      selectedFolderId: null,
      filterUntagged: false,
      searchQuery: "",
      sortBy: "modification",
      sortOrder: "desc"
    });
  },

  hasActiveFilters: () => {
    return filterState.selectedTags.length > 0 || 
           filterState.filterUntagged || 
           filterState.selectedFolderId !== null;
  }
};

export { filterState };
