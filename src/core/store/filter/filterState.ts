import { createStore } from 'solid-js/store';
import { APP_CONFIG } from '../../../config/constants';
import { type SearchGroup } from './schemas';

export type SortField =
    | 'modified_at'
    | 'added_at'
    | 'created_at'
    | 'filename'
    | 'format'
    | 'size'
    | 'rating';
export type SortOrder = 'asc' | 'desc';
export type ViewLayout = 'masonry-v' | 'masonry-h' | 'grid' | 'list';

export interface FilterSnapshot {
    selectedTags: string[];
    selectedFolderIds: string[];
    folderRecursiveView: boolean;
    filterUntagged: boolean;
    filterHasTags: boolean;
    filterFavorites: boolean;
    filterTrash: boolean;
    filterFamily: string | null;
    searchQuery: string;
    /** Whether to use fuzzy matching for the search query */
    searchFuzzy: boolean;
    advancedSearch: SearchGroup | null;
    sortBy: SortField;
    sortOrder: SortOrder;
}

export interface FilterState extends FilterSnapshot {
    layout: ViewLayout;
    thumbSize: number;

    // History
    history: FilterSnapshot[];
    historyIndex: number;
    historyLimit: number;
}

export const STORAGE_KEY = 'mundam-filter-preference';

export const defaultSnapshot: FilterSnapshot = {
    selectedTags: [],
    selectedFolderIds: [],
    folderRecursiveView: false,
    filterUntagged: false,
    filterHasTags: false,
    filterFavorites: false,
    filterTrash: false,
    filterFamily: null,
    searchQuery: '',
    searchFuzzy: false,
    advancedSearch: null,
    sortBy: 'modified_at',
    sortOrder: 'desc'
};

const defaultState: FilterState = {
    ...defaultSnapshot,
    layout: 'masonry-v',
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

export const [filterState, setFilterState] = createStore<FilterState>({
    ...defaultState,
    ...persisted,
    // Don't persist these
    selectedTags: [],
    selectedFolderIds: [],
    filterUntagged: false,
    filterHasTags: false,
    filterFavorites: false,
    filterTrash: false,
    searchQuery: '',
    searchFuzzy: false,
    advancedSearch: null,
    history: [{ ...defaultSnapshot }],
    historyIndex: 0
});

export const filterStateInternal = { setFilterState };

export const persist = (newState: Partial<FilterState>) => {
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
