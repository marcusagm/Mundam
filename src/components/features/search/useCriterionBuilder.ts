import { createSignal, createMemo, createEffect } from 'solid-js';
import { type SearchCriterion, filterActions } from '../../../core/store/filter';
import { SEARCH_FIELDS, OPERATORS_FOR_TYPE } from '../../../core/store/filter/constants';
import { supportedFormats } from '../../../core/store/systemStore';

export type SearchValue = string | number | null | Date;

/**
 * Manages the state of a single "add criterion" form. Each search group owns its own instance,
 * so partially filled forms of different groups never interfere with each other.
 *
 * @param metadata - Contextual information used to pick sensible default values.
 * @returns Reactive state accessors, mutators and a `buildCriterion` action.
 */
export const useCriterionBuilder = (metadata: { locations: { id: string; name: string }[] }) => {
    const [currentKey, setCurrentKey] = createSignal('tags');
    const [currentOperator, setCurrentOperator] = createSignal('contains');
    const [currentValue, setCurrentValue] = createSignal<SearchValue>(null);
    const [currentValue2, setCurrentValue2] = createSignal<SearchValue>(null);
    const [currentUnitMultiplier, setCurrentUnitMultiplier] = createSignal('1048576');
    const [validationErrors, setValidationErrors] = createSignal<Record<string, string>>({});

    /** Derived signal identifying the currently selected field configuration. */
    const selectedField = createMemo(() =>
        SEARCH_FIELDS.find(field => field.value === currentKey())
    );

    /** Derived list of operators valid for the currently selected field's data type. */
    const availableOperators = createMemo(() => {
        const field = selectedField();
        return field ? OPERATORS_FOR_TYPE[field.type] || [] : [];
    });

    /** Automatically resets the operator and values when the search key (field) changes. */
    createEffect(() => {
        const field = selectedField();
        if (!field) return;

        setCurrentOperator(OPERATORS_FOR_TYPE[field.type]?.[0]?.value || '');

        if (field.type === 'rating') {
            setCurrentValue(0);
        } else if (field.type === 'select') {
            setCurrentValue(supportedFormats()[0]?.extensions[0] || '');
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
    });

    /**
     * Validates the form and, when valid, returns the new criterion and clears the values.
     *
     * @returns The built criterion, or `null` when validation failed.
     */
    const buildCriterion = (): SearchCriterion | null => {
        const activeErrors = filterActions.validateCriterion(
            currentKey(),
            currentOperator(),
            currentValue(),
            currentValue2(),
            currentUnitMultiplier()
        );
        setValidationErrors(activeErrors);
        if (Object.keys(activeErrors).length > 0) return null;

        const { finalValue, unitMultiplier } = filterActions.processCriterion(
            currentKey(),
            currentOperator(),
            currentValue(),
            currentValue2(),
            currentUnitMultiplier()
        );

        const criterion: SearchCriterion = {
            // Random ids avoid collisions with criteria loaded from saved smart folders.
            id: `criterion-${crypto.randomUUID()}`,
            key: currentKey(),
            operator: currentOperator(),
            value: finalValue as SearchCriterion['value'],
            unitMultiplier
        };
        criterion.displayValue = filterActions.formatCriterionDisplay(criterion);

        setCurrentValue(null);
        setCurrentValue2(null);
        return criterion;
    };

    return {
        currentKey,
        currentOperator,
        currentValue,
        currentValue2,
        currentUnitMultiplier,
        validationErrors,
        selectedField,
        availableOperators,
        setCurrentKey,
        setCurrentOperator,
        setCurrentValue,
        setCurrentValue2,
        setCurrentUnitMultiplier,
        setValidationErrors,
        buildCriterion
    };
};
