import { createSignal, createMemo, createEffect } from 'solid-js';
import {
    type SearchCriterion,
    type LogicalOperator,
    type SearchGroup,
    filterActions
} from '../../../core/store/filter';
import { SEARCH_FIELDS, OPERATORS_FOR_TYPE } from '../../../core/store/filter/constants';
import { fromISO } from '../../../utils/format';
import { supportedFormats } from '../../../core/store/systemStore';

export type SearchValue = string | number | null | Date;
type SearchItem = SearchGroup['items'][number];

const updateCriteriaTree = (
    items: SearchItem[],
    criterionId: string,
    update: (criterion: SearchCriterion) => SearchCriterion
): SearchItem[] =>
    items.map(item => {
        if ('key' in item) return item.id === criterionId ? update(item) : item;
        return { ...item, items: updateCriteriaTree(item.items, criterionId, update) };
    });

const removeCriteriaTree = (items: SearchItem[], criterionId: string): SearchItem[] =>
    items.reduce<SearchItem[]>((remaining, item) => {
        if ('key' in item) {
            if (item.id !== criterionId) remaining.push(item);
            return remaining;
        }

        const remainingItems = removeCriteriaTree(item.items, criterionId);
        if (remainingItems.length > 0) remaining.push({ ...item, items: remainingItems });
        return remaining;
    }, []);

const findCriterion = (items: SearchItem[], criterionId: string): SearchCriterion | undefined => {
    for (const item of items) {
        if ('key' in item && item.id === criterionId) return item;
        if (!('key' in item)) {
            const match = findCriterion(item.items, criterionId);
            if (match) return match;
        }
    }
    return undefined;
};

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
export const useAdvancedSearch = (
    metadata: {
        locations: { id: string; name: string }[];
        tags: { id: string; name: string }[];
    },
    queryOptions: {
        /** Function that returns whether the advanced search panel is currently open. */
        isOpen: () => boolean;
        /** Function that returns the initial search group to populate the editor. */
        initialQuery: () => SearchGroup | undefined;
    }
) => {
    // Top-level state management (local for the editor session)
    const [criteria, setCriteria] = createSignal<SearchItem[]>([]);
    const [matchMode, setMatchMode] = createSignal<LogicalOperator>('and');

    // New criteria builder (Add mode) state
    const [currentKey, setCurrentKey] = createSignal('tags');
    const [currentOperator, setCurrentOperator] = createSignal('contains');
    const [currentValue, setCurrentValue] = createSignal<SearchValue>(null);
    const [currentValue2, setCurrentValue2] = createSignal<SearchValue>(null);
    const [currentUnitMultiplier, setCurrentUnitMultiplier] = createSignal('1048576');
    const [validationErrors, setValidationErrors] = createSignal<Record<string, string>>({});

    // Existing criteria editing state
    const [editingId, setEditingId] = createSignal<string | null>(null);
    const [editingValue, setEditingValue] = createSignal<SearchValue>(null);
    const [editingValue2, setEditingValue2] = createSignal<SearchValue>(null);
    const [editingUnitMultiplier, setEditingUnitMultiplier] = createSignal('1048576');
    const [editingValidationErrors, setEditingValidationErrors] = createSignal<
        Record<string, string>
    >({});

    /** Derived signal identifying the currently selected field configuration. */
    const selectedField = createMemo(() =>
        SEARCH_FIELDS.find(field => field.value === currentKey())
    );

    /** Derived list of operators valid for the currently selected field's data type. */
    const availableOperators = createMemo(() => {
        const field = selectedField();
        return field ? OPERATORS_FOR_TYPE[field.type] || [] : [];
    });

    /** Synchronizes internal state with the provided initial query whenever the panel opens. */
    createEffect(() => {
        if (queryOptions.isOpen()) {
            const initialQuery = queryOptions.initialQuery();
            if (initialQuery) {
                setMatchMode(initialQuery.logicalOperator);
                setCriteria(initialQuery.items);
            } else {
                setCriteria([]);
                setMatchMode('and');
            }
            // Reset temporary builder and editing states
            setCurrentValue(null);
            setCurrentValue2(null);
            setValidationErrors({});
            setEditingValidationErrors({});
            setEditingId(null);
        }
    });

    /** Automatically resets the operator and values when the search key (field) changes. */
    createEffect(() => {
        const field = selectedField();
        if (field) {
            const defaultComparisonOperator = OPERATORS_FOR_TYPE[field.type]?.[0]?.value;
            setCurrentOperator(defaultComparisonOperator || '');

            // Synchronize initial state values with UI visual expectations.
            if (field.type === 'rating') {
                setCurrentValue(0);
            } else if (field.type === 'select') {
                const firstAvailableFormat = supportedFormats()[0]?.extensions[0];
                setCurrentValue(firstAvailableFormat || '');
            } else if (field.type === 'folder') {
                const firstLocationId = metadata.locations[0]?.id;
                setCurrentValue(firstLocationId ? Number(firstLocationId) : null);
            } else if (field.type === 'number' || field.type === 'size') {
                setCurrentValue(1);
            } else {
                setCurrentValue(null);
            }

            setCurrentValue2(null);
            setValidationErrors({});
            setEditingValidationErrors({});
        }
    });

    /** Validates the current 'Add mode' builder state using store logic. */
    const validateCurrentBuilderState = () => {
        const activeErrors = filterActions.validateCriterion(
            currentKey(),
            currentOperator(),
            currentValue(),
            currentValue2(),
            currentUnitMultiplier()
        );
        setValidationErrors(activeErrors);
        return Object.keys(activeErrors).length === 0;
    };

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

        const originalCriterion = findCriterion(criteria(), activeEditingId);
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

        setCriteria(previousItems =>
            updateCriteriaTree(previousItems, activeEditingId, () => ({
                ...partiallyUpdatedCriterion,
                displayValue
            }))
        );
        setEditingId(null);
        setEditingValidationErrors({});
    };

    /** Adds the currently constructed criterion from the builder into the active list. */
    const handleAddCriteria = () => {
        if (!validateCurrentBuilderState()) return;

        const { finalValue, unitMultiplier } = filterActions.processCriterion(
            currentKey(),
            currentOperator(),
            currentValue(),
            currentValue2(),
            currentUnitMultiplier()
        );

        const newCriterionObject: SearchCriterion = {
            id: `temp_${Math.random().toString(36).substr(2, 9)}`, // Temporary local ID
            key: currentKey(),
            operator: currentOperator(),
            value: finalValue as SearchCriterion['value'],
            unitMultiplier,
            displayValue: '' // Will be updated below
        };

        newCriterionObject.displayValue = filterActions.formatCriterionDisplay(newCriterionObject);

        setCriteria([...criteria(), newCriterionObject]);

        // Clean up builder values after successful addition
        setCurrentValue(null);
        setCurrentValue2(null);
    };

    /**
     * Removes a specific criterion from the active search list.
     * @param criterionId - The unique identifier of the criterion to be deleted.
     */
    const handleRemoveCriteria = (criterionId: string) => {
        setCriteria(removeCriteriaTree(criteria(), criterionId));
    };

    /** Wipes all defined criteria and resets match mode to default. */
    const handleResetAllCriteria = () => {
        setCriteria([]);
        setMatchMode('and');
    };

    return {
        // Reactive State Accessors
        criteria,
        matchMode,
        currentKey,
        currentOperator,
        currentValue,
        currentValue2,
        currentUnitMultiplier,
        validationErrors,
        editingId,
        editingValue,
        editingValue2,
        editingUnitMultiplier,
        editingValidationErrors,
        selectedField,
        availableOperators,

        // Direct State Mutators
        setCriteria,
        setMatchMode,
        setCurrentKey,
        setCurrentOperator,
        setCurrentValue,
        setCurrentValue2,
        setCurrentUnitMultiplier,
        setValidationErrors,
        setEditingId,
        setEditingValue,
        setEditingValue2,
        setEditingUnitMultiplier,
        setEditingValidationErrors,

        // Business Logic Actions
        handleStartEdit,
        handleConfirmEdit,
        handleAddCriteria,
        handleRemoveCriteria,
        handleReset: handleResetAllCriteria
    };
};
