import { createStore } from "solid-js/store";
import { batch } from "solid-js";
import { APP_CONFIG } from "../../config/constants";

export type SortField = "modified_at" | "added_at" | "created_at" | "filename" | "format" | "size" | "rating";
export type SortOrder = "asc" | "desc";
export type ViewLayout = "masonry-v" | "masonry-h" | "grid" | "list";

export type LogicalOperator = "and" | "or";

export interface SearchCriterion {
  id: string;
  key: string;
  operator: string;
  value: any;
  unitMultiplier?: string;
  displayValue?: string;
}

export interface SearchGroup {
  id: string;
  logicalOperator: LogicalOperator;
  items: (SearchCriterion | SearchGroup)[];
}

interface FilterSnapshot {
  selectedTags: number[];
  selectedFolderId: number | null;
  folderRecursiveView: boolean;
  filterUntagged: boolean;
  searchQuery: string;
  advancedSearch: SearchGroup | null;
  sortBy: SortField;
  sortOrder: SortOrder;
}

interface FilterState extends FilterSnapshot {
  layout: ViewLayout;
  thumbSize: number;
  
  // History
  history: FilterSnapshot[];
  historyIndex: number;
  historyLimit: number;
}

const STORAGE_KEY = "mundam-filter-preference";
// const HISTORY_LIMIT = 50; // Moved to state

const defaultSnapshot: FilterSnapshot = {
  selectedTags: [],
  selectedFolderId: null,
  folderRecursiveView: false,
  filterUntagged: false,
  searchQuery: "",
  advancedSearch: null,
  sortBy: "modified_at",
  sortOrder: "desc",
};

const defaultState: FilterState = {
  ...defaultSnapshot,
  layout: "masonry-v",
  thumbSize: APP_CONFIG.THUMBNAIL_SIZE,
  history: [{ ...defaultSnapshot }],
  historyIndex: 0,
  historyLimit: 50 // Default
};

const getPersisted = (): Partial<FilterState> => {
    try {
        const saved = localStorage.getItem(STORAGE_KEY);
        return saved ? JSON.parse(saved) : {};
    } catch {
        return {};
    }
};

const persisted = getPersisted();

const [filterState, setFilterState] = createStore<FilterState>({
  ...defaultState,
  ...persisted,
  // Don't persist these
  selectedTags: [],
  selectedFolderId: null,
  filterUntagged: false,
  searchQuery: "",
  advancedSearch: null,
  history: [{ ...defaultSnapshot }],
  historyIndex: 0
});

const persist = (newState: Partial<FilterState>) => {
    const toSave = {
        sortBy: filterState.sortBy,
        sortOrder: filterState.sortOrder,
        layout: filterState.layout,
        thumbSize: filterState.thumbSize,
        historyLimit: filterState.historyLimit,
        ...newState
    };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(toSave));
};

let searchDebounceTimer: any;

const filterActions = {
  pushHistory: () => {
    const snapshot: FilterSnapshot = {
      selectedTags: filterState.selectedTags,
      selectedFolderId: filterState.selectedFolderId,
      folderRecursiveView: filterState.folderRecursiveView,
      filterUntagged: filterState.filterUntagged,
      searchQuery: filterState.searchQuery,
      advancedSearch: filterState.advancedSearch,
      sortBy: filterState.sortBy,
      sortOrder: filterState.sortOrder,
    };

    // Check if the current state is different from the last history item
    const current = filterState.history[filterState.historyIndex];
    const isSame = JSON.stringify(snapshot) === JSON.stringify(current);
    
    if (isSame) return;

    const newHistory = filterState.history.slice(0, filterState.historyIndex + 1);
    newHistory.push(snapshot);
    
    // Limit history
    const limit = filterState.historyLimit || 50;
    const finalHistory = newHistory.length > limit 
      ? newHistory.slice(newHistory.length - limit)
      : newHistory;

    setFilterState({
      history: finalHistory,
      historyIndex: finalHistory.length - 1
    });
  },

  setHistoryLimit: (limit: number) => {
      setFilterState("historyLimit", limit);
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
  canGoForward: () => filterState.historyIndex < filterState.history.length - 1,

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
    filterActions.pushHistory();
  },

  setUntagged: (isActive: boolean) => {
    setFilterState("filterUntagged", isActive);
    if (isActive) {
      setFilterState("selectedTags", []);
    }
    filterActions.pushHistory();
  },

  toggleUntagged: () => {
    filterActions.setUntagged(!filterState.filterUntagged);
  },

  setFolder: (folderId: number | null) => {
    setFilterState("selectedFolderId", folderId);
    filterActions.pushHistory();
  },

  setFolderRecursiveView: (isRecursive: boolean) => {
    setFilterState("folderRecursiveView", isRecursive);
    filterActions.pushHistory();
  },

  setSearch: (query: string) => {
    setFilterState("searchQuery", query);
    
    // Debounce history push for search
    if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
    searchDebounceTimer = setTimeout(() => {
      filterActions.pushHistory();
    }, APP_CONFIG.SEARCH_DEBOUNCE_MS);
  },

  setAdvancedSearch: (search: SearchGroup | null) => {
    setFilterState("advancedSearch", search);
    filterActions.pushHistory();
  },

  setSortBy: (field: SortField) => {
    setFilterState("sortBy", field);
    persist({ sortBy: field });
    filterActions.pushHistory();
  },

  setSortOrder: (order: SortOrder) => {
    setFilterState("sortOrder", order);
    persist({ sortOrder: order });
    filterActions.pushHistory();
  },

  setLayout: (layout: ViewLayout) => {
    setFilterState("layout", layout);
    persist({ layout: layout });
    // Layout and zoom don't go to history as per user request
  },

  setThumbSize: (size: number) => {
    setFilterState("thumbSize", size);
    persist({ thumbSize: size });
  },

  clearAll: () => {
    setFilterState({
      selectedTags: [],
      selectedFolderId: null,
      filterUntagged: false,
      searchQuery: "",
      advancedSearch: null
    });
    filterActions.pushHistory();
  },

  hasActiveFilters: () => {
    return filterState.selectedTags.length > 0 || 
           filterState.filterUntagged || 
           filterState.selectedFolderId !== null ||
           filterState.searchQuery !== "" ||
           filterState.advancedSearch !== null;
  }
};

export { filterState, filterActions };
