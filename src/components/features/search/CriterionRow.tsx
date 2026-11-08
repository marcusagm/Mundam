import { Component, Show, createMemo } from 'solid-js';
import { Dynamic } from 'solid-js/web';
import {
    Tag,
    Palette,
    Folder,
    Calendar,
    Star,
    HardDrive,
    Maximize2,
    FileCode,
    FileText,
    Pencil,
    Trash2,
    Check,
    X
} from 'lucide-solid';
import { Button } from '../../ui';
import { SEARCH_FIELDS, OPERATORS_FOR_TYPE } from '../../../core/store/filter/constants';
import { type SearchCriterion } from '../../../core/store/filter';
import { cn } from '../../../lib/utils';
import { criterionHandlerRegistry } from './fields';
import { CriterionFieldRendererProperties } from './fields/types';
import { useAdvancedSearch, SearchValue } from './useAdvancedSearch';

/**
 * Properties for the CriterionRow component.
 */
interface CriterionRowProperties {
    /** The individual search criterion to be displayed or edited. */
    criterion: SearchCriterion;
    /** The active advanced search context containing mutators and editing state. */
    search: ReturnType<typeof useAdvancedSearch>;
    /** Callback to remove this criterion from its parent group. */
    onRemove: () => void;
}

const renderFieldCategoryIcon = (fieldType: string) => {
    switch (fieldType) {
        case 'tags':
            return <Tag size={13} />;
        case 'color':
            return <Palette size={13} />;
        case 'folder':
            return <Folder size={13} />;
        case 'date':
            return <Calendar size={13} />;
        case 'rating':
            return <Star size={13} />;
        case 'size':
            return <HardDrive size={13} />;
        case 'number':
            return <Maximize2 size={13} />;
        case 'select':
            return <FileCode size={13} />;
        default:
            return <FileText size={13} />;
    }
};

/**
 * Displays a single search rule with thematic icon, pill operator, and rich formatted value.
 * Supports inline editing with clear confirmation and cancellation actions.
 *
 * @param properties - Properties for the component.
 * @returns The rendered rule row.
 */
export const CriterionRow: Component<CriterionRowProperties> = properties => {
    const fieldConfiguration = createMemo(() =>
        SEARCH_FIELDS.find(item => item.value === properties.criterion.key)
    );

    const isCurrentlyEditing = () => properties.search.editingId() === properties.criterion.id;

    const operatorLabel = createMemo(() => {
        const fieldType = fieldConfiguration()?.type || 'text';
        const operatorList = OPERATORS_FOR_TYPE[fieldType] || [];
        const matchedOperator = operatorList.find(
            operatorOption => operatorOption.value === properties.criterion.operator
        );
        return matchedOperator?.label || properties.criterion.operator;
    });

    const DynamicFieldComponent = createMemo(() => {
        const fieldType = fieldConfiguration()?.type || 'text';
        const handler = criterionHandlerRegistry[fieldType] || criterionHandlerRegistry.text;
        return handler.component;
    });

    const handleCancelEdit = () => {
        properties.search.setEditingId(null);
        properties.search.setEditingValidationErrors({});
    };

    return (
        <div
            class={cn('criterion-row', isCurrentlyEditing() && 'is-editing')}
            role="listitem"
            aria-label={`Rule: ${fieldConfiguration()?.label || properties.criterion.key} ${operatorLabel()} ${properties.criterion.displayValue}`}
        >
            <div class="criterion-row-leading">
                <span class="criterion-category-icon" aria-hidden="true">
                    {renderFieldCategoryIcon(fieldConfiguration()?.type || 'text')}
                </span>
                <span class="criterion-field-name">
                    {fieldConfiguration()?.label || properties.criterion.key}
                </span>
                <span class="criterion-operator-badge">{operatorLabel()}</span>
            </div>

            <div class="criterion-row-value-container">
                <Show
                    when={!isCurrentlyEditing()}
                    fallback={
                        <div class="criterion-edit-inputs">
                            <Dynamic
                                component={
                                    DynamicFieldComponent() as unknown as Component<CriterionFieldRendererProperties>
                                }
                                fieldKey={properties.criterion.key}
                                comparisonOperator={properties.criterion.operator}
                                value={properties.search.editingValue()}
                                setValue={(newValue: SearchValue) => {
                                    properties.search.setEditingValue(() => newValue);
                                    if (properties.search.editingValidationErrors().value) {
                                        properties.search.setEditingValidationErrors(
                                            (previousErrors: Record<string, string>) => ({
                                                ...previousErrors,
                                                value: ''
                                            })
                                        );
                                    }
                                }}
                                value2={properties.search.editingValue2()}
                                setValue2={(newValue2: SearchValue) => {
                                    properties.search.setEditingValue2(() => newValue2);
                                    if (properties.search.editingValidationErrors().value2) {
                                        properties.search.setEditingValidationErrors(
                                            (previousErrors: Record<string, string>) => ({
                                                ...previousErrors,
                                                value2: ''
                                            })
                                        );
                                    }
                                }}
                                unitMultiplier={properties.search.editingUnitMultiplier()}
                                setUnitMultiplier={(newMultiplier: string) => {
                                    properties.search.setEditingUnitMultiplier(newMultiplier);
                                    if (properties.search.editingValidationErrors().unit) {
                                        properties.search.setEditingValidationErrors(
                                            (previousErrors: Record<string, string>) => ({
                                                ...previousErrors,
                                                unit: ''
                                            })
                                        );
                                    }
                                }}
                                errors={properties.search.editingValidationErrors()}
                                size="sm"
                            />
                        </div>
                    }
                >
                    <span class="criterion-value-badge">{properties.criterion.displayValue}</span>
                </Show>
            </div>

            <div class="criterion-row-actions">
                <Show
                    when={!isCurrentlyEditing()}
                    fallback={
                        <>
                            <Button
                                variant="primary"
                                size="icon-xs"
                                onClick={properties.search.handleConfirmEdit}
                                title="Confirm changes"
                                aria-label="Confirm changes"
                            >
                                <Check size={14} />
                            </Button>
                            <Button
                                variant="ghost"
                                size="icon-xs"
                                onClick={handleCancelEdit}
                                title="Cancel editing"
                                aria-label="Cancel editing"
                            >
                                <X size={14} />
                            </Button>
                        </>
                    }
                >
                    <Button
                        variant="ghost"
                        size="icon-xs"
                        onClick={() => properties.search.handleStartEdit(properties.criterion)}
                        title="Edit rule"
                        aria-label="Edit rule"
                    >
                        <Pencil size={13} />
                    </Button>
                    <Button
                        variant="ghost-destructive"
                        size="icon-xs"
                        onClick={properties.onRemove}
                        title="Remove rule"
                        aria-label="Remove rule"
                    >
                        <Trash2 size={13} />
                    </Button>
                </Show>
            </div>
        </div>
    );
};
