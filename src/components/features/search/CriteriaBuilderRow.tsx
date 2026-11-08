import { Component, createMemo, Show } from 'solid-js';
import { Dynamic } from 'solid-js/web';
import { Plus, X } from 'lucide-solid';
import { Button } from '../../ui';
import { Select } from '../../ui/Select';
import { useMetadata } from '../../../core/hooks';
import { type SearchCriterion } from '../../../core/store/filter';
import { SEARCH_FIELDS } from '../../../core/store/filter/constants';
import { criterionHandlerRegistry } from './fields';
import { useCriterionBuilder, SearchValue } from './useCriterionBuilder';

/**
 * Properties for the CriteriaBuilderRow component.
 */
interface CriteriaBuilderRowProperties {
    /** Callback invoked with a fully validated criterion when confirmed. */
    onAdd: (newCriterion: SearchCriterion) => void;
    /** Optional callback invoked when the user cancels the builder form. */
    onCancel?: () => void;
}

/**
 * Inline composer row for creating a new search rule within a group.
 * Maintains isolated validation and resets fields upon submission.
 *
 * @param properties - Properties for the component.
 * @returns The rendered builder form row.
 */
export const CriteriaBuilderRow: Component<CriteriaBuilderRowProperties> = properties => {
    const metadataState = useMetadata();
    const criterionBuilder = useCriterionBuilder(metadataState);

    const DynamicFieldComponent = createMemo(() => {
        const fieldType = criterionBuilder.selectedField()?.type;
        const handler =
            criterionHandlerRegistry[fieldType || 'text'] || criterionHandlerRegistry.text;
        return handler.component;
    });

    const handleConfirmAdd = () => {
        const validatedCriterion = criterionBuilder.buildCriterion();
        if (validatedCriterion) {
            properties.onAdd(validatedCriterion);
        }
    };

    return (
        <div class="criteria-builder-row-container" role="region" aria-label="Add new condition">
            <div class="criteria-builder-inputs-grid">
                <div class="criteria-builder-field-select">
                    <Select
                        options={SEARCH_FIELDS}
                        value={criterionBuilder.currentKey()}
                        onValueChange={criterionBuilder.setCurrentKey}
                        size="sm"
                        placeholder="Select field..."
                    />
                </div>

                <div class="criteria-builder-operator-select">
                    <Select
                        options={criterionBuilder.availableOperators()}
                        value={criterionBuilder.currentOperator()}
                        onValueChange={criterionBuilder.setCurrentOperator}
                        size="sm"
                        placeholder="Select operator..."
                    />
                </div>

                <div class="criteria-builder-value-field">
                    <Dynamic
                        component={DynamicFieldComponent()}
                        fieldKey={criterionBuilder.currentKey()}
                        comparisonOperator={criterionBuilder.currentOperator()}
                        value={criterionBuilder.currentValue()}
                        setValue={(newValue: SearchValue) => {
                            criterionBuilder.setCurrentValue(() => newValue);
                            if (criterionBuilder.validationErrors().value) {
                                criterionBuilder.setValidationErrors(
                                    (previousErrors: Record<string, string>) => ({
                                        ...previousErrors,
                                        value: ''
                                    })
                                );
                            }
                        }}
                        value2={criterionBuilder.currentValue2()}
                        setValue2={(newValue2: SearchValue) => {
                            criterionBuilder.setCurrentValue2(() => newValue2);
                            if (criterionBuilder.validationErrors().value2) {
                                criterionBuilder.setValidationErrors(
                                    (previousErrors: Record<string, string>) => ({
                                        ...previousErrors,
                                        value2: ''
                                    })
                                );
                            }
                        }}
                        unitMultiplier={criterionBuilder.currentUnitMultiplier()}
                        setUnitMultiplier={(newMultiplier: string) => {
                            criterionBuilder.setCurrentUnitMultiplier(newMultiplier);
                            if (criterionBuilder.validationErrors().unit) {
                                criterionBuilder.setValidationErrors(
                                    (previousErrors: Record<string, string>) => ({
                                        ...previousErrors,
                                        unit: ''
                                    })
                                );
                            }
                        }}
                        errors={criterionBuilder.validationErrors()}
                        size="sm"
                    />
                </div>
            </div>

            <div class="criteria-builder-action-buttons">
                <Button
                    variant="primary"
                    size="sm"
                    onClick={handleConfirmAdd}
                    leftIcon={<Plus size={14} />}
                >
                    Add Condition
                </Button>

                <Show when={properties.onCancel}>
                    <Button
                        variant="ghost"
                        size="sm"
                        onClick={properties.onCancel}
                        leftIcon={<X size={14} />}
                    >
                        Cancel
                    </Button>
                </Show>
            </div>
        </div>
    );
};
