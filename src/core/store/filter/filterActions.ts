import { batch } from 'solid-js';
import { APP_CONFIG } from '../../../config/constants';
import {
    SearchGroupSchema,
    type SearchCriterion,
    type SearchGroup,
    type LogicalOperator
} from './schemas';
import { ActionResult, ErrorCode } from '../../types/actions';
import { createId } from '../../../lib/primitives/createId';
import {
    filterState,
    filterStateInternal,
    persist,
    type SortField,
    type SortOrder,
    type ViewLayout
} from './filterState';
import { historyActions } from './historyActions';
import { criterionHelpers } from './criterionHelpers';

const { setFilterState } = filterStateInternal;

let searchDebounceTimer: ReturnType<typeof setTimeout> | undefined;

type QuickFilterKey = 'filterUntagged' | 'filterHasTags' | 'filterFavorites' | 'filterTrash';

function toggleQuickFilter(filterKey: QuickFilterKey, isAdditive: boolean): void {
    const shouldActivate = !filterState[filterKey];

    batch(() => {
        if (shouldActivate && !isAdditive) {
            setFilterState({
                filterUntagged: filterKey === 'filterUntagged',
                filterHasTags: filterKey === 'filterHasTags',
                filterFavorites: filterKey === 'filterFavorites',
                filterTrash: filterKey === 'filterTrash'
            });
        } else {
            setFilterState(filterKey, shouldActivate);
        }

        if (shouldActivate && (filterKey === 'filterUntagged' || filterKey === 'filterHasTags')) {
            setFilterState('selectedTags', []);
            if (filterKey === 'filterUntagged') setFilterState('filterHasTags', false);
            if (filterKey === 'filterHasTags') setFilterState('filterUntagged', false);
        }
    });

    historyActions.pushHistory();
}

export const filterActions = {
    ...historyActions,
    ...criterionHelpers,

    toggleTag: (tagId: string) => {
        const current = filterState.selectedTags;
        if (current.includes(tagId)) {
            setFilterState('selectedTags', tags => tags.filter(id => id !== tagId));
        } else {
            setFilterState({
                selectedTags: [...current, tagId],
                filterUntagged: false
            });
        }
        historyActions.pushHistory();
    },

    setUntagged: (isActive: boolean) => {
        setFilterState('filterUntagged', isActive);
        if (isActive) {
            setFilterState('selectedTags', []);
        }
        historyActions.pushHistory();
    },

    toggleUntagged: (isAdditive = false) => {
        toggleQuickFilter('filterUntagged', isAdditive);
    },

    setHasTags: (isActive: boolean) => {
        setFilterState('filterHasTags', isActive);
        if (isActive) {
            setFilterState('selectedTags', []);
            setFilterState('filterUntagged', false);
        }
        historyActions.pushHistory();
    },

    toggleHasTags: (isAdditive = false) => {
        toggleQuickFilter('filterHasTags', isAdditive);
    },

    setFavorites: (isActive: boolean) => {
        setFilterState('filterFavorites', isActive);
        historyActions.pushHistory();
    },

    toggleFavorites: (isAdditive = false) => {
        toggleQuickFilter('filterFavorites', isAdditive);
    },

    setTrash: (isActive: boolean) => {
        setFilterState('filterTrash', isActive);
        historyActions.pushHistory();
    },

    toggleTrash: (isAdditive = false) => {
        toggleQuickFilter('filterTrash', isAdditive);
    },

    setFamily: (family: string | null) => {
        setFilterState('filterFamily', family);
        historyActions.pushHistory();
    },

    setFolder: (folderId: string | null) => {
        setFilterState('selectedFolderId', folderId);
        historyActions.pushHistory();
    },

    setFolderRecursiveView: (isRecursive: boolean) => {
        setFilterState('folderRecursiveView', isRecursive);
        historyActions.pushHistory();
    },

    setSearch: (query: string) => {
        setFilterState('searchQuery', query);

        // Debounce history push for search
        if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
        searchDebounceTimer = setTimeout(() => {
            historyActions.pushHistory();
        }, APP_CONFIG.SEARCH_DEBOUNCE_MS);
    },

    /** Enables or disables fuzzy matching for the search query and saves to history */
    setSearchFuzzy: (isFuzzy: boolean) => {
        setFilterState('searchFuzzy', isFuzzy);
        historyActions.pushHistory();
    },

    setAdvancedSearch: (search: SearchGroup | null): ActionResult => {
        if (search) {
            const result = SearchGroupSchema.safeParse(search);
            if (!result.success) {
                return {
                    success: false,
                    error: {
                        code: ErrorCode.VALIDATION_ERROR,
                        message: 'Invalid search group structure',
                        details: result.error.format()
                    }
                };
            }
        }

        setFilterState('advancedSearch', search);
        historyActions.pushHistory();
        return { success: true, data: undefined };
    },

    addCriterion: (
        criterion: Omit<SearchCriterion, 'id' | 'displayValue'>
    ): ActionResult<string> => {
        const errors = criterionHelpers.validateCriterion(
            criterion.key,
            criterion.operator,
            criterion.value,
            undefined, // value2 should be handled by logic.process before calling this
            criterion.unitMultiplier
        );

        if (Object.keys(errors).length > 0) {
            return {
                success: false,
                error: {
                    code: ErrorCode.VALIDATION_ERROR,
                    message: 'Invalid criterion values',
                    details: errors
                }
            };
        }

        const id = createId('criterion');
        const displayValue = criterionHelpers.formatCriterionDisplay(criterion);
        const newCriterion: SearchCriterion = { ...criterion, id, displayValue };

        let currentGroup = filterState.advancedSearch;
        if (!currentGroup) {
            currentGroup = { id: createId('group'), logicalOperator: 'and', items: [] };
        }

        const newGroup: SearchGroup = {
            ...currentGroup,
            items: [...currentGroup.items, newCriterion]
        };

        const result = filterActions.setAdvancedSearch(newGroup);
        if (!result.success) return result;

        return { success: true, data: id };
    },

    removeCriterion: (id: string) => {
        const currentGroup = filterState.advancedSearch;
        if (!currentGroup) return;

        const newItems = currentGroup.items.filter((item: SearchCriterion | SearchGroup) => {
            if ('id' in item) return item.id !== id;
            return true;
        });

        filterActions.setAdvancedSearch({ ...currentGroup, items: newItems });
    },

    updateCriterion: (id: string, updates: Partial<SearchCriterion>): ActionResult => {
        const currentGroup = filterState.advancedSearch;
        if (!currentGroup)
            return {
                success: false,
                error: { code: ErrorCode.VALIDATION_ERROR, message: 'No search group active' }
            };

        const newItems = currentGroup.items.map((item: SearchCriterion | SearchGroup) => {
            if ('key' in item && item.id === id) {
                const merged = { ...item, ...updates } as SearchCriterion;
                const displayValue = criterionHelpers.formatCriterionDisplay(merged);
                return { ...merged, displayValue };
            }
            return item;
        });

        return filterActions.setAdvancedSearch({ ...currentGroup, items: newItems });
    },

    setMatchMode: (mode: LogicalOperator) => {
        const currentGroup = filterState.advancedSearch;
        if (currentGroup) {
            filterActions.setAdvancedSearch({ ...currentGroup, logicalOperator: mode });
        } else {
            filterActions.setAdvancedSearch({
                id: createId('group'),
                logicalOperator: mode,
                items: []
            });
        }
    },

    setSortBy: (field: SortField) => {
        setFilterState('sortBy', field);
        persist({ sortBy: field });
        historyActions.pushHistory();
    },

    setSortOrder: (order: SortOrder) => {
        setFilterState('sortOrder', order);
        persist({ sortOrder: order });
        historyActions.pushHistory();
    },

    setLayout: (layout: ViewLayout) => {
        setFilterState('layout', layout);
        persist({ layout: layout });
        // Layout and zoom don't go to history as per user request
    },

    setThumbSize: (size: number) => {
        setFilterState('thumbSize', size);
        persist({ thumbSize: size });
    },

    clearAll: () => {
        setFilterState({
            selectedTags: [],
            selectedFolderId: null,
            filterUntagged: false,
            filterHasTags: false,
            filterFavorites: false,
            filterTrash: false,
            filterFamily: null,
            searchQuery: '',
            searchFuzzy: false,
            advancedSearch: null
        });
        historyActions.pushHistory();
    },

    hasActiveFilters: () => {
        return (
            filterState.selectedTags.length > 0 ||
            filterState.filterUntagged ||
            filterState.filterHasTags ||
            filterState.filterFavorites ||
            filterState.filterTrash ||
            filterState.filterFamily !== null ||
            filterState.selectedFolderId !== null ||
            filterState.searchQuery !== '' ||
            filterState.advancedSearch !== null
        );
    }
};
