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

/** Quick filters are mutually exclusive: activating one deactivates the others. */
function toggleQuickFilter(filterKey: QuickFilterKey): void {
    const shouldActivate = !filterState[filterKey];

    batch(() => {
        setFilterState({
            filterUntagged: shouldActivate && filterKey === 'filterUntagged',
            filterHasTags: shouldActivate && filterKey === 'filterHasTags',
            filterFavorites: shouldActivate && filterKey === 'filterFavorites',
            filterTrash: shouldActivate && filterKey === 'filterTrash'
        });

        if (shouldActivate && (filterKey === 'filterUntagged' || filterKey === 'filterHasTags')) {
            setFilterState('selectedTags', []);
        }
    });

    historyActions.pushHistory();
}

/** Toggles `id` inside `list`, adding or removing it. */
function toggleInList(list: string[], id: string): string[] {
    return list.includes(id) ? list.filter(item => item !== id) : [...list, id];
}

/** Ordinary selection replaces the list; clicking the only selected item clears it. */
function replaceInList(list: string[], id: string): string[] {
    return list.length === 1 && list[0] === id ? [] : [id];
}

export const filterActions = {
    ...historyActions,
    ...criterionHelpers,

    /**
     * Selects a tag. Ordinary selection replaces the current tags;
     * additive selection (Ctrl/Meta+click) toggles the tag in the selection.
     */
    selectTag: (tagId: string, isAdditive = false) => {
        const next = isAdditive
            ? toggleInList(filterState.selectedTags, tagId)
            : replaceInList(filterState.selectedTags, tagId);
        batch(() => {
            setFilterState('selectedTags', next);
            if (next.includes(tagId)) setFilterState('filterUntagged', false);
        });
        historyActions.pushHistory();
    },

    toggleTag: (tagId: string) => {
        filterActions.selectTag(tagId, true);
    },

    setUntagged: (isActive: boolean) => {
        setFilterState('filterUntagged', isActive);
        if (isActive) {
            setFilterState('selectedTags', []);
        }
        historyActions.pushHistory();
    },

    toggleUntagged: () => {
        toggleQuickFilter('filterUntagged');
    },

    setHasTags: (isActive: boolean) => {
        setFilterState('filterHasTags', isActive);
        if (isActive) {
            setFilterState('selectedTags', []);
            setFilterState('filterUntagged', false);
        }
        historyActions.pushHistory();
    },

    toggleHasTags: () => {
        toggleQuickFilter('filterHasTags');
    },

    setFavorites: (isActive: boolean) => {
        setFilterState('filterFavorites', isActive);
        historyActions.pushHistory();
    },

    toggleFavorites: () => {
        toggleQuickFilter('filterFavorites');
    },

    setTrash: (isActive: boolean) => {
        setFilterState('filterTrash', isActive);
        historyActions.pushHistory();
    },

    toggleTrash: () => {
        toggleQuickFilter('filterTrash');
    },

    setFamily: (family: string | null) => {
        setFilterState('filterFamily', family);
        historyActions.pushHistory();
    },

    /**
     * Selects a folder. Ordinary selection replaces the current folders;
     * additive selection (Ctrl/Meta+click) toggles the folder in the selection.
     */
    selectFolder: (folderId: string, isAdditive = false) => {
        setFilterState(
            'selectedFolderIds',
            isAdditive
                ? toggleInList(filterState.selectedFolderIds, folderId)
                : replaceInList(filterState.selectedFolderIds, folderId)
        );
        historyActions.pushHistory();
    },

    clearFolders: () => {
        setFilterState('selectedFolderIds', []);
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
            selectedFolderIds: [],
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
            filterState.selectedFolderIds.length > 0 ||
            filterState.searchQuery !== '' ||
            filterState.advancedSearch !== null
        );
    }
};
