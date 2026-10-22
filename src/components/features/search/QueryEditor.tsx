import { Button } from '../../ui';
import { Component, For, Show, createMemo } from 'solid-js';
import { Dynamic } from 'solid-js/web';
import { Info, Pencil, Check, Trash2, CircleQuestionMark } from 'lucide-solid';
import { RadioGroup, RadioGroupItem } from '../../ui/RadioGroup';
import { Tooltip } from '../../ui/Tooltip';
import { SEARCH_FIELDS, OPERATORS_FOR_TYPE } from '../../../core/store/filter/constants';
import {
    type LogicalOperator,
    type SearchCriterion,
    type SearchGroup
} from '../../../core/store/filter';
import { cn } from '../../../lib/utils';
import { criterionHandlerRegistry } from './fields';
import { CriterionFieldRendererProperties } from './fields/types';
import { useAdvancedSearch, SearchValue } from './useAdvancedSearch';

export interface QueryEditorProps {
    search: ReturnType<typeof useAdvancedSearch>;
}

const CriterionItem: Component<{
    item: SearchCriterion;
    index: number;
    search: ReturnType<typeof useAdvancedSearch>;
}> = props => {
    const field = createMemo(() => SEARCH_FIELDS.find(f => f.value === props.item.key));
    const isEditing = () => props.search.editingId() === props.item.id;

    const DynamicFieldComponent = createMemo(() => {
        const type = field()?.type;
        const handler = criterionHandlerRegistry[type || 'text'] || criterionHandlerRegistry.text;
        return handler.component;
    });

    return (
        <div class={cn('criterion-item', isEditing() && 'editing')}>
            <span class="criterion-index">{props.index + 1}</span>
            <span class="criterion-field">{field()?.label || props.item.key}</span>
            <span class="criterion-operator">
                {OPERATORS_FOR_TYPE[field()?.type || '']?.find(o => o.value === props.item.operator)
                    ?.label || props.item.operator}
            </span>
            <span class="criterion-value">
                <Show
                    when={!isEditing()}
                    fallback={
                        <div class="edit-inputs">
                            <Dynamic
                                component={
                                    DynamicFieldComponent() as unknown as Component<CriterionFieldRendererProperties>
                                }
                                fieldKey={props.item.key}
                                comparisonOperator={props.item.operator}
                                value={props.search.editingValue()}
                                setValue={(val: SearchValue) => {
                                    props.search.setEditingValue(() => val);
                                    if (props.search.editingValidationErrors().value) {
                                        props.search.setEditingValidationErrors(
                                            (prev: Record<string, string>) => ({
                                                ...prev,
                                                value: ''
                                            })
                                        );
                                    }
                                }}
                                value2={props.search.editingValue2()}
                                setValue2={(val: SearchValue) => {
                                    props.search.setEditingValue2(() => val);
                                    if (props.search.editingValidationErrors().value2) {
                                        props.search.setEditingValidationErrors(
                                            (prev: Record<string, string>) => ({
                                                ...prev,
                                                value2: ''
                                            })
                                        );
                                    }
                                }}
                                unitMultiplier={props.search.editingUnitMultiplier()}
                                setUnitMultiplier={(unit: string) => {
                                    props.search.setEditingUnitMultiplier(unit);
                                    if (props.search.editingValidationErrors().unit) {
                                        props.search.setEditingValidationErrors(
                                            (prev: Record<string, string>) => ({
                                                ...prev,
                                                unit: ''
                                            })
                                        );
                                    }
                                }}
                                errors={props.search.editingValidationErrors()}
                                size="sm"
                            />
                        </div>
                    }
                >
                    {props.item.displayValue}
                </Show>
            </span>
            <Show
                when={!isEditing()}
                fallback={
                    <Button variant="ghost" size="icon" onClick={props.search.handleConfirmEdit}>
                        <Check size={16} />
                    </Button>
                }
            >
                <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => props.search.handleStartEdit(props.item)}
                >
                    <Pencil size={14} />
                </Button>
            </Show>
            <Button
                variant="ghost-destructive"
                size="icon"
                onClick={() => props.search.handleRemoveCriteria(props.item.id)}
            >
                <Trash2 size={14} />
            </Button>
        </div>
    );
};

const QueryItems: Component<{
    items: SearchGroup['items'];
    search: ReturnType<typeof useAdvancedSearch>;
}> = props => (
    <For each={props.items}>
        {(item, index) =>
            'key' in item ? (
                <CriterionItem item={item} index={index()} search={props.search} />
            ) : (
                <div class="criterion-group">
                    <div class="criterion-group-heading">
                        {item.logicalOperator === 'and' ? 'All of' : 'Any of'}
                    </div>
                    <QueryItems items={item.items} search={props.search} />
                </div>
            )
        }
    </For>
);

export const QueryEditor: Component<QueryEditorProps> = props => {
    return (
        <div class="query-editor-section">
            <div class="section-title">
                Query Editor
                <Tooltip content="Review and manage your active criteria. You can edit values in-line or remove them. All criteria work together based on the 'Any' or 'All' match mode below.">
                    <CircleQuestionMark size={12} />
                </Tooltip>
            </div>
            <div class="criteria-list">
                <Show when={props.search.criteria().length === 0}>
                    <div class="empty-query-info">
                        <Info size={24} />
                        <div>
                            <strong>Empty Query</strong>
                            <br />
                            Your query is currently empty. Create a criteria above to enable search.
                        </div>
                    </div>
                </Show>
                <QueryItems items={props.search.criteria()} search={props.search} />
            </div>

            <div class="match-mode-section">
                <div class="section-title">
                    Match Mode
                    <Tooltip content="Choose how to combine your criteria. 'All' requires every condition to be met, while 'Any' matches if at least one condition is met.">
                        <CircleQuestionMark size={12} />
                    </Tooltip>
                </div>
                <RadioGroup
                    value={props.search.matchMode()}
                    onValueChange={val => props.search.setMatchMode(val as LogicalOperator)}
                    orientation="horizontal"
                    class="match-radio-horizontal"
                >
                    <RadioGroupItem value="or" label="Any (OR)" />
                    <RadioGroupItem value="and" label="All (AND)" />
                </RadioGroup>
            </div>
        </div>
    );
};
