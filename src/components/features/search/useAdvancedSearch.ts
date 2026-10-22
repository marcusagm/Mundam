import { createSignal, createEffect } from 'solid-js';
import {
    type SearchCriterion,
    type LogicalOperator,
    type SearchGroup,
    filterActions
} from '../../../core/store/filter';
import { fromISO } from '../../../utils/format';
import type { SearchValue } from './useCriterionBuilder';
import {
    addItemToGroup,
    countCriteria,
    createEmptyGroup,
    findCriterion,
    pruneEmptyGroups,
    removeCriterion,
    removeGroup,
    setGroupOperator,
    updateCriterion,
    type SearchItem
} from './searchTree';

export type { SearchValue };

const extractArrayValue = (
    key: string,
    value: unknown[],
    unitMultiplier: string
): [SearchValue, SearchValue] => {
    if (key === 'size') {
        const multiplier = Number(unitMultiplier);
        return [Number(value[0]) / multiplier, Number(value[1]) / multiplier];
    }
    if (['added_at', 'created_at', 'modified_at'].includes(key)) {
        return [
            fromISO(String(value[0])) as unknown as SearchValue,
            fromISO(String(value[1])) as unknown as SearchValue
        ];
    }
    return [value[0] as SearchValue, value[1] as SearchValue];
};

const extractSingleValue = (key: string, value: unknown, unitMultiplier: string): SearchValue => {
    if (key === 'size') {
        return Number(value) / Number(unitMultiplier);
    }
    if (['added_at', 'created_at', 'modified_at'].includes(key)) {
        return fromISO(String(value)) as unknown as SearchValue;
    }
    if (key === 'color') {
        if (typeof value === 'object' && value !== null) {
            const colorObject = value as Record<string, unknown>;
            const threshold = (colorObject.threshold as number) ?? 25;
            const proximity = Math.max(
                0,
                Math.min(100, Math.round(((threshold - 2.3) / (50 - 2.3)) * 100))
            );
            return JSON.stringify({ hex: colorObject.hex, proximity });
        }
    }
    return value as SearchValue;
};

/**
 * Custom hook that manages the complex state and logic for the Advanced Search system.
 * Handles the construction, validation, and editing of multiple search criteria,
 * relying on filterStore actions for domain logic and validation.
 *
 * @param metadata - Contextual information from stores (e.g., tags, locations).
 * @param queryOptions - Configuration for visibility and initial query state.
 * @returns An object containing reactive state accessors and action handlers.
 */
export const useAdvancedSearch = (queryOptions: {
    /** Function that returns whether the advanced search panel is currently open. */
    isOpen: () => boolean;
    /** Function that returns the initial search group to populate the editor. */
    initialQuery: () => SearchGroup | undefined;
}) => {
    /** Root group of the query. It always exists and can never be removed. */
    const [rootGroup, setRootGroup] = createSignal<SearchGroup>(createEmptyGroup());

    // Existing criteria editing state (only one criterion is edited at a time)
    const [editingId, setEditingId] = createSignal<string | null>(null);
    const [editingValue, setEditingValue] = createSignal<SearchValue>(null);
    const [editingValue2, setEditingValue2] = createSignal<SearchValue>(null);
    const [editingUnitMultiplier, setEditingUnitMultiplier] = createSignal('1048576');
    const [editingValidationErrors, setEditingValidationErrors] = createSignal<
        Record<string, string>
    >({});

    /** Synchronizes internal state with the provided initial query whenever the panel opens. */
    createEffect(() => {
        if (queryOptions.isOpen()) {
            setRootGroup(queryOptions.initialQuery() ?? createEmptyGroup());
            setEditingValidationErrors({});
            setEditingId(null);
        }
    });

    /**
     * Activates the editing mode for a specific existing criterion.
     * @param criterionItem - The specific search criterion to be edited.
     */
    const handleStartEdit = (criterionItem: SearchCriterion) => {
        setEditingId(criterionItem.id);
        setEditingValidationErrors({});
        const multiplier = criterionItem.unitMultiplier || '1048576';
        setEditingUnitMultiplier(multiplier);

        if (Array.isArray(criterionItem.value)) {
            const [val1, val2] = extractArrayValue(
                criterionItem.key,
                criterionItem.value,
                multiplier
            );
            setEditingValue(val1);
            setEditingValue2(val2);
        } else {
            setEditingValue(extractSingleValue(criterionItem.key, criterionItem.value, multiplier));
            setEditingValue2(null);
        }
    };

    /** Confirms changes made in the editing session and updates the main criteria list. */
    const handleConfirmEdit = () => {
        const activeEditingId = editingId();
        if (!activeEditingId) return;

        const originalCriterion = findCriterion(rootGroup(), activeEditingId);
        if (!originalCriterion) return;

        const activeErrors = filterActions.validateCriterion(
            originalCriterion.key,
            originalCriterion.operator,
            editingValue(),
            editingValue2(),
            editingUnitMultiplier()
        );

        if (Object.keys(activeErrors).length > 0) {
            setEditingValidationErrors(activeErrors);
            return;
        }

        const { finalValue, unitMultiplier } = filterActions.processCriterion(
            originalCriterion.key,
            originalCriterion.operator,
            editingValue(),
            editingValue2(),
            editingUnitMultiplier()
        );

        const partiallyUpdatedCriterion = {
            ...originalCriterion,
            value: finalValue as SearchCriterion['value'],
            unitMultiplier
        };

        const displayValue = filterActions.formatCriterionDisplay(partiallyUpdatedCriterion);

        setRootGroup(previousRoot =>
            updateCriterion(previousRoot, activeEditingId, () => ({
                ...partiallyUpdatedCriterion,
                displayValue
            }))
        );
        setEditingId(null);
        setEditingValidationErrors({});
    };

    /** Adds a criterion or a nested group to the group identified by `groupId`. */
    const handleAddItem = (groupId: string, item: SearchItem) => {
        setRootGroup(previousRoot => addItemToGroup(previousRoot, groupId, item));
    };

    /** Adds a new empty group inside the group identified by `parentGroupId`. */
    const handleAddGroup = (parentGroupId: string, logicalOperator: LogicalOperator = 'and') => {
        handleAddItem(parentGroupId, createEmptyGroup(logicalOperator));
    };

    /** Removes a nested group. The root group is protected and ignored. */
    const handleRemoveGroup = (groupId: string) => {
        setRootGroup(previousRoot => removeGroup(previousRoot, groupId));
    };

    /** Changes how the items of a group are combined (AND/OR). */
    const handleSetGroupOperator = (groupId: string, logicalOperator: LogicalOperator) => {
        setRootGroup(previousRoot => setGroupOperator(previousRoot, groupId, logicalOperator));
    };

    /**
     * Removes a specific criterion from the query.
     * @param criterionId - The unique identifier of the criterion to be deleted.
     */
    const handleRemoveCriteria = (criterionId: string) => {
        if (editingId() === criterionId) setEditingId(null);
        setRootGroup(previousRoot => removeCriterion(previousRoot, criterionId));
    };

    /** Wipes all criteria and nested groups, keeping an empty root group. */
    const handleResetAllCriteria = () => {
        setEditingId(null);
        setRootGroup(previousRoot => ({ ...previousRoot, logicalOperator: 'and', items: [] }));
    };

    /** Number of criteria across all groups. */
    const criteriaCount = () => countCriteria(rootGroup());

    /** Root group ready to be searched or saved, without empty groups. */
    const buildQuery = (): SearchGroup => pruneEmptyGroups(rootGroup());

    return {
        // Reactive State Accessors
        rootGroup,
        criteriaCount,
        editingId,
        editingValue,
        editingValue2,
        editingUnitMultiplier,
        editingValidationErrors,

        // Direct State Mutators
        setEditingId,
        setEditingValue,
        setEditingValue2,
        setEditingUnitMultiplier,
        setEditingValidationErrors,

        // Business Logic Actions
        buildQuery,
        handleStartEdit,
        handleConfirmEdit,
        handleAddItem,
        handleAddGroup,
        handleRemoveGroup,
        handleSetGroupOperator,
        handleRemoveCriteria,
        handleReset: handleResetAllCriteria
    };
};
