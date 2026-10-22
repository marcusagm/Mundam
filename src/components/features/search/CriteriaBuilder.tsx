import { Button } from '../../ui';
import { Component, createMemo } from 'solid-js';
import { Dynamic } from 'solid-js/web';
import { Plus, CircleQuestionMark } from 'lucide-solid';
import { Select } from '../../ui/Select';
import { Tooltip } from '../../ui/Tooltip';
import { useMetadata } from '../../../core/hooks';
import { type SearchCriterion } from '../../../core/store/filter';
import { SEARCH_FIELDS } from '../../../core/store/filter/constants';
import { criterionHandlerRegistry } from './fields';
import { useCriterionBuilder, SearchValue } from './useCriterionBuilder';

export interface CriteriaBuilderProps {
    /** Called with a validated criterion when the user confirms the form. */
    onAdd: (criterion: SearchCriterion) => void;
}

/**
 * Form for building a single criterion. It keeps its own state, so several
 * instances (one per search group) can coexist.
 */
export const CriteriaBuilder: Component<CriteriaBuilderProps> = props => {
    const metadata = useMetadata();
    const builder = useCriterionBuilder(metadata);

    const DynamicFieldComponent = createMemo(() => {
        const type = builder.selectedField()?.type;
        const handler = criterionHandlerRegistry[type || 'text'] || criterionHandlerRegistry.text;
        return handler.component;
    });

    const handleAdd = () => {
        const criterion = builder.buildCriterion();
        if (criterion) props.onAdd(criterion);
    };

    return (
        <div class="criteria-builder-section">
            <div class="section-title">
                Criteria Builder
                <Tooltip content="Choose a field, operator, and value to add new search criteria. Filter by name, tags, date, and more.">
                    <CircleQuestionMark size={12} />
                </Tooltip>
            </div>
            <div class="builder-row">
                <Select
                    options={SEARCH_FIELDS}
                    value={builder.currentKey()}
                    onValueChange={builder.setCurrentKey}
                />
                <Select
                    options={builder.availableOperators()}
                    value={builder.currentOperator()}
                    onValueChange={builder.setCurrentOperator}
                />

                <div class="builder-value-field">
                    <Dynamic
                        component={DynamicFieldComponent()}
                        fieldKey={builder.currentKey()}
                        comparisonOperator={builder.currentOperator()}
                        value={builder.currentValue()}
                        setValue={(val: SearchValue) => {
                            builder.setCurrentValue(() => val);
                            if (builder.validationErrors().value) {
                                builder.setValidationErrors((prev: Record<string, string>) => ({
                                    ...prev,
                                    value: ''
                                }));
                            }
                        }}
                        value2={builder.currentValue2()}
                        setValue2={(val: SearchValue) => {
                            builder.setCurrentValue2(() => val);
                            if (builder.validationErrors().value2) {
                                builder.setValidationErrors((prev: Record<string, string>) => ({
                                    ...prev,
                                    value2: ''
                                }));
                            }
                        }}
                        unitMultiplier={builder.currentUnitMultiplier()}
                        setUnitMultiplier={(unit: string) => {
                            builder.setCurrentUnitMultiplier(unit);
                            if (builder.validationErrors().unit) {
                                builder.setValidationErrors((prev: Record<string, string>) => ({
                                    ...prev,
                                    unit: ''
                                }));
                            }
                        }}
                        errors={builder.validationErrors()}
                    />
                </div>

                <div class="builder-actions">
                    <Button variant="ghost" size="icon" onClick={handleAdd} class="add-button">
                        <Plus />
                    </Button>
                </div>
            </div>
        </div>
    );
};
