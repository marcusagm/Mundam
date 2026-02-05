import { Component, createSignal, Show, For, createMemo, createEffect } from 'solid-js';
import { supportedFormats } from '../../../core/store/systemStore';
import { Plus, Trash2, Search, Save, CircleQuestionMark, Info, Pencil, Check } from 'lucide-solid';
import { Modal } from '../../ui/Modal';
import { Button } from '../../ui/Button';
import { Input } from '../../ui/Input';
import { Select } from '../../ui/Select';
import { RadioGroup, RadioGroupItem } from '../../ui/RadioGroup';
import { Tooltip } from '../../ui/Tooltip';
import { useFilters, useMetadata } from '../../../core/hooks';
import { SearchCriterion, SearchGroup, LogicalOperator } from '../../../core/store/filterStore';
import { createId } from '../../../lib/primitives/createId';
import { NumberInput } from '../../ui/NumberInput';
import { DateInput } from '../../ui/DateInput';
import { cn } from '../../../lib/utils';
import './advanced-search-modal.css';

// --- Helpers ---

const formatToISO = (val: any) => {
    if (val instanceof Date) {
        const y = val.getFullYear();
        const m = (val.getMonth() + 1).toString().padStart(2, '0');
        const d = val.getDate().toString().padStart(2, '0');
        return `${y}-${m}-${d}`;
    }
    return val;
};

const formatToDisplay = (iso: string) => {
    if (!iso || typeof iso !== 'string') return iso;
    const parts = iso.split('-');
    if (parts.length === 3) {
        const d = new Date(parseInt(parts[0]), parseInt(parts[1]) - 1, parseInt(parts[2]));
        if (!isNaN(d.getTime())) {
            const day = d.getDate().toString().padStart(2, '0');
            const month = (d.getMonth() + 1).toString().padStart(2, '0');
            const year = d.getFullYear();
            return `${day}/${month}/${year}`;
        }
    }
    return iso;
};

const fromISO = (iso: string) => {
    if (!iso || typeof iso !== 'string') return null;
    const parts = iso.split('-');
    if (parts.length === 3) {
        const d = new Date(parseInt(parts[0]), parseInt(parts[1]) - 1, parseInt(parts[2]));
        return isNaN(d.getTime()) ? null : d;
    }
    return null;
};

const SIZE_UNITS = [
    { value: '1', label: 'Bytes' },
    { value: '1024', label: 'KB' },
    { value: '1048576', label: 'MB' },
    { value: '1073741824', label: 'GB' }
];

const computeDisplayValue = (item: Partial<SearchCriterion>, metadata: any): string => {
    if (item.displayValue) return item.displayValue;
    if (item.value === null || item.value === undefined) return '';

    const key = item.key || '';
    const val = item.value;

    if (key === 'size') {
        const m = Number(item.unitMultiplier || '1048576');
        const label = SIZE_UNITS.find(u => u.value === String(m))?.label || 'MB';
        if (Array.isArray(val)) {
            return `${val[0] / m} ${label} to ${val[1] / m} ${label}`;
        }
        return `${val / m} ${label}`;
    }

    if (['added_at', 'created_at', 'modified_at'].includes(key)) {
        if (Array.isArray(val)) {
            return `${formatToDisplay(val[0])} to ${formatToDisplay(val[1])}`;
        }
        return formatToDisplay(String(val));
    }

    if (key === 'folder') {
        return (
            metadata.locations.find((l: any) => String(l.id) === String(val))?.name || String(val)
        );
    }

    if (key === 'tags') {
        return metadata.tags.find((t: any) => String(t.id) === String(val))?.name || String(val);
    }

    if (Array.isArray(val)) {
        return `${val[0]} to ${val[1]}`;
    }

    return String(val);
};

interface AdvancedSearchModalProps {
    isOpen: boolean;
    onClose: () => void;
    isSmartFolderMode?: boolean;
    initialId?: number;
    initialName?: string;
    initialQuery?: SearchGroup;
    onSave?: (name: string, query: SearchGroup, id?: number) => void;
}

const SEARCH_FIELDS = [
    { value: 'tags', label: 'Tags', type: 'tags' },
    { value: 'filename', label: 'Filename', type: 'text' },
    { value: 'format', label: 'Format', type: 'select' },
    { value: 'size', label: 'File size', type: 'number' },
    { value: 'width', label: 'Width', type: 'number' },
    { value: 'height', label: 'Height', type: 'number' },
    { value: 'added_at', label: 'Date added', type: 'date' },
    { value: 'created_at', label: 'Date creation', type: 'date' },
    { value: 'modified_at', label: 'Date modified', type: 'date' },
    { value: 'rating', label: 'Rating', type: 'rating' },
    { value: 'notes', label: 'Notes', type: 'text' },
    { value: 'folder', label: 'Folder', type: 'folder' }
];

const OPERATORS_FOR_TYPE: Record<string, { value: string; label: string }[]> = {
    text: [
        { value: 'contains', label: 'Contains' },
        { value: 'not_contains', label: 'Not Contains' },
        { value: 'equals', label: 'Equals' },
        { value: 'starts_with', label: 'Starts With' },
        { value: 'ends_with', label: 'Ends With' }
    ],
    number: [
        { value: 'gt', label: 'Greater than' },
        { value: 'lt', label: 'Less than' },
        { value: 'eq', label: 'Equals' },
        { value: 'between', label: 'Between' }
    ],
    date: [
        { value: 'before', label: 'Before' },
        { value: 'after', label: 'After' },
        { value: 'on', label: 'On' },
        { value: 'between', label: 'Between' }
    ],
    select: [
        { value: 'eq', label: 'Equals' },
        { value: 'neq', label: 'Not Equals' }
    ],
    tags: [
        { value: 'contains', label: 'Contains' },
        { value: 'not_contains', label: 'Not Contains' }
    ],
    folder: [
        { value: 'is', label: 'Is' },
        { value: 'in', label: 'Is inside (recursive)' }
    ],
    rating: [
        { value: 'eq', label: 'Equals' },
        { value: 'gte', label: 'Greater than or equal' },
        { value: 'lte', label: 'Less than or equal' }
    ]
};

export const AdvancedSearchModal: Component<AdvancedSearchModalProps> = props => {
    const filters = useFilters();
    const metadata = useMetadata();

    const [smartFolderName, setSmartFolderName] = createSignal(props.initialName || '');
    const [currentKey, setCurrentKey] = createSignal('tags');
    const [currentOperator, setCurrentOperator] = createSignal('contains');
    const [currentValue, setCurrentValue] = createSignal<any>(null);
    const [currentUnit, setCurrentUnit] = createSignal('1048576');
    const [currentValue2, setCurrentValue2] = createSignal<any>(null);
    const [criteria, setCriteria] = createSignal<SearchCriterion[]>([]);
    const [matchMode, setMatchMode] = createSignal<LogicalOperator>('and');
    const [validationErrors, setValidationErrors] = createSignal<Record<string, string>>({});

    // Editing state
    const [editingId, setEditingId] = createSignal<string | null>(null);
    const [editingValue, setEditingValue] = createSignal<any>(null);
    const [editingValue2, setEditingValue2] = createSignal<any>(null);
    const [editingUnit, setEditingUnit] = createSignal('1048576');
    const [editingValidationErrors, setEditingValidationErrors] = createSignal<
        Record<string, string>
    >({});

    // Initialize from props when opening
    createEffect(() => {
        if (props.isOpen) {
            setSmartFolderName(props.initialName || '');
            if (props.initialQuery) {
                setMatchMode(props.initialQuery.logicalOperator);
                // Filter and hydrate criteria
                const initialCriteria = props.initialQuery.items
                    .filter(item => !('items' in item))
                    .map(item => ({
                        ...(item as SearchCriterion),
                        displayValue:
                            (item as SearchCriterion).displayValue ||
                            computeDisplayValue(item as SearchCriterion, metadata)
                    }));
                setCriteria(initialCriteria);
            } else {
                setCriteria([]);
                setMatchMode('and');
            }
        }
    });

    const selectedField = createMemo(() => SEARCH_FIELDS.find(f => f.value === currentKey()));
    const availableOperators = createMemo(() => {
        const field = selectedField();
        return field ? OPERATORS_FOR_TYPE[field.type] || [] : [];
    });

    // Reset operator when key changes
    createEffect(() => {
        const field = selectedField();
        if (field) {
            const defaultOp = OPERATORS_FOR_TYPE[field.type]?.[0]?.value;
            setCurrentOperator(defaultOp || '');
            setCurrentValue(null);
            setCurrentValue2(null);
            setValidationErrors({});
            setEditingValidationErrors({});
        }
    });

    const validateCriterion = (field: any, op: string, val: any, val2: any, unit?: string) => {
        const errors: Record<string, string> = {};

        if (val === null || val === '') {
            errors.value = 'Value is required';
        }

        if (op === 'between' && (val2 === null || val2 === '')) {
            errors.value2 = 'End value is required';
        }

        if (field?.type === 'date') {
            if (val === null) errors.value = 'Date is required';
            if (op === 'between' && val2 === null) errors.value2 = 'End date is required';
        }

        if (field?.value === 'size') {
            if (!unit || isNaN(Number(unit)) || !SIZE_UNITS.some(u => u.value === unit)) {
                errors.unit = 'Unit is required';
            }
        }

        return errors;
    };

    const validateCurrent = () => {
        const errors = validateCriterion(
            selectedField(),
            currentOperator(),
            currentValue(),
            currentValue2(),
            currentUnit()
        );
        setValidationErrors(errors);
        return Object.keys(errors).length === 0;
    };

    const handleStartEdit = (item: SearchCriterion) => {
        setEditingId(item.id);
        setEditingValidationErrors({});
        setEditingUnit(item.unitMultiplier || '1048576');

        if (Array.isArray(item.value)) {
            if (item.key === 'size') {
                const mult = Number(item.unitMultiplier || '1048576');
                setEditingValue(Number(item.value[0]) / mult);
                setEditingValue2(Number(item.value[1]) / mult);
            } else if (['added_at', 'created_at', 'modified_at'].includes(item.key)) {
                setEditingValue(fromISO(item.value[0]));
                setEditingValue2(fromISO(item.value[1]));
            } else {
                setEditingValue(item.value[0]);
                setEditingValue2(item.value[1]);
            }
        } else {
            if (item.key === 'size') {
                const mult = Number(item.unitMultiplier || '1048576');
                setEditingValue(Number(item.value) / mult);
            } else if (['added_at', 'created_at', 'modified_at'].includes(item.key)) {
                setEditingValue(fromISO(String(item.value)));
            } else {
                setEditingValue(item.value);
            }
            setEditingValue2(null);
        }
    };

    const handleConfirmEdit = () => {
        const id = editingId();
        if (!id) return;

        const currentItem = criteria().find(c => c.id === id);
        if (!currentItem) return;

        const field = SEARCH_FIELDS.find(f => f.value === currentItem.key);
        const errors = validateCriterion(
            field,
            currentItem.operator,
            editingValue(),
            editingValue2(),
            editingUnit()
        );

        if (Object.keys(errors).length > 0) {
            setEditingValidationErrors(errors);
            return;
        }

        setCriteria(prev =>
            prev.map(c => {
                if (c.id === id) {
                    let finalValue = editingValue();
                    let displayValue: string | undefined;

                    // Handle size conversion
                    if (c.key === 'size') {
                        const multiplier = Number(editingUnit());
                        const label =
                            SIZE_UNITS.find(u => u.value === editingUnit())?.label || 'MB';
                        if (c.operator === 'between') {
                            const v1 = Math.round(Number(editingValue()) * multiplier);
                            const v2 = Math.round(Number(editingValue2()) * multiplier);
                            finalValue = [v1, v2];
                            displayValue = `${editingValue()} ${label} to ${editingValue2()} ${label}`;
                        } else {
                            finalValue = Math.round(Number(editingValue()) * multiplier);
                            displayValue = `${editingValue()} ${label}`;
                        }
                    } else if (c.operator === 'between') {
                        if (['added_at', 'created_at', 'modified_at'].includes(c.key)) {
                            const v1 = formatToISO(editingValue());
                            const v2 = formatToISO(editingValue2());
                            finalValue = [v1, v2];
                            displayValue = `${formatToDisplay(v1)} to ${formatToDisplay(v2)}`;
                        } else {
                            finalValue = [editingValue(), editingValue2()];
                            displayValue = `${editingValue()} to ${editingValue2()}`;
                        }
                    } else if (['added_at', 'created_at', 'modified_at'].includes(c.key)) {
                        finalValue = formatToISO(editingValue());
                        displayValue = formatToDisplay(finalValue);
                    } else if (c.key === 'folder') {
                        displayValue =
                            metadata.locations.find(l => String(l.id) === String(editingValue()))
                                ?.name || String(editingValue());
                    } else if (c.key === 'tags') {
                        displayValue =
                            metadata.tags.find(t => String(t.id) === String(editingValue()))
                                ?.name || String(editingValue());
                    }

                    return {
                        ...c,
                        value: finalValue,
                        displayValue,
                        unitMultiplier: c.key === 'size' ? editingUnit() : undefined
                    };
                }
                return c;
            })
        );
        setEditingId(null);
        setEditingValidationErrors({});
    };

    const handleAddCriteria = () => {
        if (!validateCurrent()) return;

        // Date conversion logic is handled per-operator below

        let finalValue = currentValue();
        let displayValue: string | undefined;

        // Handle Unit conversion for size
        if (currentKey() === 'size' && finalValue !== null) {
            const multiplier = Number(currentUnit());
            const label = SIZE_UNITS.find(u => u.value === currentUnit())?.label || 'MB';
            if (currentOperator() === 'between') {
                const v1 = Math.round(Number(finalValue) * multiplier);
                const v2 = Math.round(Number(currentValue2()) * multiplier);
                finalValue = [v1, v2];
                displayValue = `${currentValue()} ${label} to ${currentValue2()} ${label}`;
            } else {
                finalValue = Math.round(Number(finalValue) * multiplier);
                displayValue = `${currentValue()} ${label}`;
            }
        } else if (currentOperator() === 'between') {
            if (selectedField()?.type === 'date') {
                const v1 = formatToISO(currentValue());
                const v2 = formatToISO(currentValue2());
                finalValue = [v1, v2];
                displayValue = `${formatToDisplay(v1)} to ${formatToDisplay(v2)}`;
            } else {
                finalValue = [currentValue(), currentValue2()];
                displayValue = `${currentValue()} to ${currentValue2()}`;
            }
        } else if (selectedField()?.type === 'date') {
            finalValue = formatToISO(currentValue());
            displayValue = formatToDisplay(finalValue);
        } else if (currentKey() === 'folder') {
            displayValue =
                metadata.locations.find(l => String(l.id) === String(currentValue()))?.name ||
                String(currentValue());
        } else if (currentKey() === 'tags') {
            displayValue =
                metadata.tags.find(t => String(t.id) === String(currentValue()))?.name ||
                String(currentValue());
        }

        const newCriterion: SearchCriterion = {
            id: createId('criterion'),
            key: currentKey(),
            operator: currentOperator(),
            value: finalValue,
            displayValue,
            unitMultiplier: currentKey() === 'size' ? currentUnit() : undefined
        };
        setCriteria([...criteria(), newCriterion]);
        // Reset current values
        setCurrentValue(null);
        setCurrentValue2(null);
    };

    const handleRemoveCriteria = (id: string) => {
        setCriteria(criteria().filter(c => c.id !== id));
    };

    const handleReset = () => {
        setCriteria([]);
        setMatchMode('and');
    };

    const handleSearch = () => {
        const searchGroup: SearchGroup = {
            id: createId('group'),
            logicalOperator: matchMode(),
            items: criteria()
        };
        filters.setAdvancedSearch(searchGroup);
        props.onClose();
    };

    const handleSaveSmartFolder = () => {
        if (!smartFolderName()) {
            setValidationErrors(prev => ({ ...prev, smartFolderName: 'Name is required' }));
            return;
        }
        const searchGroup: SearchGroup = {
            id: createId('group'),
            logicalOperator: matchMode(),
            items: criteria()
        };
        props.onSave?.(smartFolderName(), searchGroup, props.initialId);
        props.onClose();
    };

    const hierarchicalTags = createMemo(() => {
        const indent = (
            tags: any[],
            parentId: number | null = null,
            depth = 0
        ): { value: string; label: string }[] => {
            return tags
                .filter(t => t.parent_id === parentId || (parentId === null && !t.parent_id))
                .flatMap(t => [
                    { value: String(t.id), label: `${'\u00A0'.repeat(depth * 3)}${t.name}` },
                    ...indent(tags, t.id, depth + 1)
                ]);
        };
        return indent(metadata.tags);
    });

    const hierarchicalFolders = createMemo(() => {
        const indent = (
            folders: any[],
            parentId: number | null = null,
            depth = 0
        ): { value: string; label: string }[] => {
            return folders
                .filter(f => f.parent_id === parentId)
                .flatMap(f => [
                    { value: String(f.id), label: `${'\u00A0'.repeat(depth * 4)}${f.name}` },
                    ...indent(folders, f.id, depth + 1)
                ]);
        };
        return indent(metadata.locations);
    });

    return (
        <Modal
            isOpen={props.isOpen}
            onClose={props.onClose}
            title={props.isSmartFolderMode ? 'Smart Folder Configuration' : 'Advanced Search'}
            size="xl"
            footer={
                <div class="modal-footer-content">
                    <Button variant="secondary" onClick={handleReset}>
                        Reset
                    </Button>
                    <div style={{ flex: 1 }} />

                    <Show when={criteria().length > 0}>
                        <Button
                            variant="secondary"
                            onClick={handleSaveSmartFolder}
                            disabled={!smartFolderName()}
                            leftIcon={<Save size={16} />}
                        >
                            Save Smart Folder
                        </Button>
                    </Show>

                    <Button
                        variant="primary"
                        onClick={handleSearch}
                        disabled={criteria().length === 0}
                        leftIcon={<Search size={16} />}
                    >
                        Search
                    </Button>
                </div>
            }
        >
            <div class="advanced-search-modal-content">
                <div class="smart-folder-name-section">
                    <Input
                        label="Smart Folder Name (optional, for saving)"
                        value={smartFolderName()}
                        onInput={e => {
                            setSmartFolderName(e.currentTarget.value);
                            if (validationErrors().smartFolderName) {
                                setValidationErrors(prev => ({ ...prev, smartFolderName: '' }));
                            }
                        }}
                        placeholder="e.g. Pictures from Tokyo"
                        error={!!validationErrors().smartFolderName}
                        errorMessage={validationErrors().smartFolderName}
                    />
                </div>

                <div class="criteria-builder-section">
                    <Tooltip content="Choose a field, operator, and value to add new search criteria. Filter by name, tags, date, and more.">
                        <div class="section-title">
                            Criteria Builder <CircleQuestionMark size={12} />
                        </div>
                    </Tooltip>
                    <div class="builder-row">
                        <Select
                            options={SEARCH_FIELDS}
                            value={currentKey()}
                            onValueChange={setCurrentKey}
                        />
                        <Select
                            options={availableOperators()}
                            value={currentOperator()}
                            onValueChange={setCurrentOperator}
                        />

                        <div class="builder-value-field">
                            {/* Render different inputs based on type */}
                            <Show when={selectedField()?.type === 'text'}>
                                <Input
                                    value={currentValue() || ''}
                                    onInput={e => {
                                        setCurrentValue(e.currentTarget.value);
                                        if (validationErrors().value)
                                            setValidationErrors(prev => ({ ...prev, value: '' }));
                                    }}
                                    placeholder="Value..."
                                    error={!!validationErrors().value}
                                    errorMessage={validationErrors().value}
                                />
                            </Show>
                            <Show when={selectedField()?.type === 'number'}>
                                <div class="number-input-group">
                                    <NumberInput
                                        value={currentValue()}
                                        onChange={val => {
                                            setCurrentValue(val);
                                            if (validationErrors().value)
                                                setValidationErrors(prev => ({
                                                    ...prev,
                                                    value: ''
                                                }));
                                        }}
                                        placeholder={
                                            currentOperator() === 'between' ? 'From...' : 'Value...'
                                        }
                                        error={!!validationErrors().value}
                                        errorMessage={validationErrors().value}
                                    />
                                    <Show when={currentOperator() === 'between'}>
                                        <span class="range-separator">to</span>
                                        <NumberInput
                                            value={currentValue2()}
                                            onChange={val => {
                                                setCurrentValue2(val);
                                                if (validationErrors().value2)
                                                    setValidationErrors(prev => ({
                                                        ...prev,
                                                        value2: ''
                                                    }));
                                            }}
                                            placeholder="To..."
                                            error={!!validationErrors().value2}
                                            errorMessage={validationErrors().value2}
                                        />
                                    </Show>
                                    <Show when={currentKey() === 'size'}>
                                        <Select
                                            class="unit-select"
                                            options={SIZE_UNITS}
                                            value={currentUnit()}
                                            onValueChange={val => {
                                                setCurrentUnit(val);
                                                if (validationErrors().unit)
                                                    setValidationErrors(prev => ({
                                                        ...prev,
                                                        unit: ''
                                                    }));
                                            }}
                                            error={!!validationErrors().unit}
                                            errorMessage={validationErrors().unit}
                                        />
                                    </Show>
                                </div>
                            </Show>
                            <Show when={selectedField()?.type === 'date'}>
                                <div class="date-input-group">
                                    <DateInput
                                        value={currentValue()}
                                        onChange={val => {
                                            setCurrentValue(val);
                                            if (validationErrors().value)
                                                setValidationErrors(prev => ({
                                                    ...prev,
                                                    value: ''
                                                }));
                                        }}
                                        placeholder={
                                            currentOperator() === 'between' ? 'From Date' : 'Date'
                                        }
                                        error={!!validationErrors().value}
                                        errorMessage={validationErrors().value}
                                    />
                                    <Show when={currentOperator() === 'between'}>
                                        <span class="range-separator">to</span>
                                        <DateInput
                                            value={currentValue2()}
                                            onChange={val => {
                                                setCurrentValue2(val);
                                                if (validationErrors().value2)
                                                    setValidationErrors(prev => ({
                                                        ...prev,
                                                        value2: ''
                                                    }));
                                            }}
                                            placeholder="To Date"
                                            error={!!validationErrors().value2}
                                            errorMessage={validationErrors().value2}
                                        />
                                    </Show>
                                </div>
                            </Show>
                            <Show when={selectedField()?.type === 'tags'}>
                                <div class="tag-select-placeholder">
                                    <Select
                                        options={hierarchicalTags()}
                                        value={String(currentValue() || '')}
                                        onValueChange={val => {
                                            setCurrentValue(val);
                                            if (validationErrors().value)
                                                setValidationErrors(prev => ({
                                                    ...prev,
                                                    value: ''
                                                }));
                                        }}
                                        placeholder="Select Tag..."
                                        searchable
                                        error={!!validationErrors().value}
                                        errorMessage={validationErrors().value}
                                    />
                                </div>
                            </Show>
                            <Show when={selectedField()?.type === 'folder'}>
                                <div class="folder-select-placeholder">
                                    <Select
                                        options={hierarchicalFolders()}
                                        value={String(currentValue() || '')}
                                        onValueChange={val => {
                                            setCurrentValue(Number(val));
                                            if (validationErrors().value)
                                                setValidationErrors(prev => ({
                                                    ...prev,
                                                    value: ''
                                                }));
                                        }}
                                        placeholder="Select Folder..."
                                        searchable
                                        error={!!validationErrors().value}
                                        errorMessage={validationErrors().value}
                                    />
                                </div>
                            </Show>
                            <Show when={selectedField()?.type === 'rating'}>
                                <Select
                                    options={[0, 1, 2, 3, 4, 5].map(v => ({
                                        value: String(v),
                                        label: `${v} Stars`
                                    }))}
                                    value={String(currentValue() || '0')}
                                    onValueChange={val => {
                                        setCurrentValue(Number(val));
                                        if (validationErrors().value)
                                            setValidationErrors(prev => ({ ...prev, value: '' }));
                                    }}
                                    error={!!validationErrors().value}
                                    errorMessage={validationErrors().value}
                                />
                            </Show>
                            <Show when={selectedField()?.type === 'select'}>
                                <Select
                                    options={supportedFormats().flatMap(f =>
                                        f.extensions.map(ext => ({
                                            value: ext,
                                            label: `${ext.toUpperCase()} - ${f.name}`
                                        }))
                                    )}
                                    value={currentValue() || ''}
                                    onValueChange={val => {
                                        setCurrentValue(val);
                                        if (validationErrors().value)
                                            setValidationErrors(prev => ({ ...prev, value: '' }));
                                    }}
                                    searchable
                                    error={!!validationErrors().value}
                                    errorMessage={validationErrors().value}
                                />
                            </Show>
                        </div>

                        <div class="builder-actions">
                            <Button
                                variant="ghost"
                                size="icon"
                                onClick={handleAddCriteria}
                                class="add-button"
                            >
                                <Plus />
                            </Button>
                        </div>
                    </div>
                </div>

                <div class="query-editor-section">
                    <Tooltip content="Review and manage your active criteria. You can edit values in-line or remove them. All criteria work together based on the 'Any' or 'All' match mode below.">
                        <div class="section-title">
                            Query Editor <CircleQuestionMark size={12} />
                        </div>
                    </Tooltip>
                    <div class="criteria-list">
                        <Show when={criteria().length === 0}>
                            <div class="empty-query-info">
                                <Info size={24} />
                                <div>
                                    <strong>Empty Query</strong>
                                    <br />
                                    Your query is currently empty. Create a criteria above to enable
                                    search.
                                </div>
                            </div>
                        </Show>
                        <For each={criteria()}>
                            {(item, index) => {
                                const field = createMemo(() =>
                                    SEARCH_FIELDS.find(f => f.value === item.key)
                                );
                                const isEditing = () => editingId() === item.id;

                                return (
                                    <div class={cn('criterion-item', isEditing() && 'editing')}>
                                        <span class="criterion-index">{index() + 1}</span>
                                        <span class="criterion-field">
                                            {field()?.label || item.key}
                                        </span>
                                        <span class="criterion-operator">
                                            {OPERATORS_FOR_TYPE[field()?.type || '']?.find(
                                                o => o.value === item.operator
                                            )?.label || item.operator}
                                        </span>
                                        <span class="criterion-value">
                                            <Show
                                                when={!isEditing()}
                                                fallback={
                                                    <div class="edit-inputs">
                                                        <Show when={field()?.type === 'text'}>
                                                            <Input
                                                                size="sm"
                                                                value={editingValue() || ''}
                                                                onInput={e => {
                                                                    setEditingValue(
                                                                        e.currentTarget.value
                                                                    );
                                                                    if (
                                                                        editingValidationErrors()
                                                                            .value
                                                                    )
                                                                        setEditingValidationErrors(
                                                                            prev => ({
                                                                                ...prev,
                                                                                value: ''
                                                                            })
                                                                        );
                                                                }}
                                                                error={
                                                                    !!editingValidationErrors()
                                                                        .value
                                                                }
                                                                errorMessage={
                                                                    editingValidationErrors().value
                                                                }
                                                            />
                                                        </Show>
                                                        <Show
                                                            when={
                                                                field()?.type === 'number' ||
                                                                item.key === 'size'
                                                            }
                                                        >
                                                            <div class="horizontal-inputs">
                                                                <NumberInput
                                                                    size="sm"
                                                                    value={editingValue()}
                                                                    onChange={val => {
                                                                        setEditingValue(val);
                                                                        if (
                                                                            editingValidationErrors()
                                                                                .value
                                                                        )
                                                                            setEditingValidationErrors(
                                                                                prev => ({
                                                                                    ...prev,
                                                                                    value: ''
                                                                                })
                                                                            );
                                                                    }}
                                                                    placeholder={
                                                                        item.operator === 'between'
                                                                            ? 'From...'
                                                                            : 'Value...'
                                                                    }
                                                                    error={
                                                                        !!editingValidationErrors()
                                                                            .value
                                                                    }
                                                                    errorMessage={
                                                                        editingValidationErrors()
                                                                            .value
                                                                    }
                                                                />
                                                                <Show
                                                                    when={
                                                                        item.operator === 'between'
                                                                    }
                                                                >
                                                                    <span>to</span>
                                                                    <NumberInput
                                                                        size="sm"
                                                                        value={editingValue2()}
                                                                        onChange={val => {
                                                                            setEditingValue2(val);
                                                                            if (
                                                                                editingValidationErrors()
                                                                                    .value2
                                                                            )
                                                                                setEditingValidationErrors(
                                                                                    prev => ({
                                                                                        ...prev,
                                                                                        value2: ''
                                                                                    })
                                                                                );
                                                                        }}
                                                                        placeholder="To..."
                                                                        error={
                                                                            !!editingValidationErrors()
                                                                                .value2
                                                                        }
                                                                        errorMessage={
                                                                            editingValidationErrors()
                                                                                .value2
                                                                        }
                                                                    />
                                                                </Show>
                                                                <Show when={item.key === 'size'}>
                                                                    <Select
                                                                        size="sm"
                                                                        class="unit-select"
                                                                        options={SIZE_UNITS}
                                                                        value={editingUnit()}
                                                                        onValueChange={val => {
                                                                            setEditingUnit(val);
                                                                            if (
                                                                                editingValidationErrors()
                                                                                    .unit
                                                                            )
                                                                                setEditingValidationErrors(
                                                                                    prev => ({
                                                                                        ...prev,
                                                                                        unit: ''
                                                                                    })
                                                                                );
                                                                        }}
                                                                        error={
                                                                            !!editingValidationErrors()
                                                                                .unit
                                                                        }
                                                                        errorMessage={
                                                                            editingValidationErrors()
                                                                                .unit
                                                                        }
                                                                    />
                                                                </Show>
                                                            </div>
                                                        </Show>
                                                        <Show when={field()?.type === 'date'}>
                                                            <div class="horizontal-inputs">
                                                                <DateInput
                                                                    size="sm"
                                                                    value={editingValue()}
                                                                    onChange={val => {
                                                                        setEditingValue(val);
                                                                        if (
                                                                            editingValidationErrors()
                                                                                .value
                                                                        )
                                                                            setEditingValidationErrors(
                                                                                prev => ({
                                                                                    ...prev,
                                                                                    value: ''
                                                                                })
                                                                            );
                                                                    }}
                                                                    placeholder={
                                                                        item.operator === 'between'
                                                                            ? 'From Date'
                                                                            : 'Date'
                                                                    }
                                                                    error={
                                                                        !!editingValidationErrors()
                                                                            .value
                                                                    }
                                                                    errorMessage={
                                                                        editingValidationErrors()
                                                                            .value
                                                                    }
                                                                />
                                                                <Show
                                                                    when={
                                                                        item.operator === 'between'
                                                                    }
                                                                >
                                                                    <span>to</span>
                                                                    <DateInput
                                                                        size="sm"
                                                                        value={editingValue2()}
                                                                        onChange={val => {
                                                                            setEditingValue2(val);
                                                                            if (
                                                                                editingValidationErrors()
                                                                                    .value2
                                                                            )
                                                                                setEditingValidationErrors(
                                                                                    prev => ({
                                                                                        ...prev,
                                                                                        value2: ''
                                                                                    })
                                                                                );
                                                                        }}
                                                                        placeholder="To Date"
                                                                        error={
                                                                            !!editingValidationErrors()
                                                                                .value2
                                                                        }
                                                                        errorMessage={
                                                                            editingValidationErrors()
                                                                                .value2
                                                                        }
                                                                    />
                                                                </Show>
                                                            </div>
                                                        </Show>
                                                        <Show when={field()?.type === 'tags'}>
                                                            <Select
                                                                size="sm"
                                                                options={hierarchicalTags()}
                                                                value={String(editingValue() || '')}
                                                                onValueChange={val => {
                                                                    setEditingValue(val);
                                                                    if (
                                                                        editingValidationErrors()
                                                                            .value
                                                                    )
                                                                        setEditingValidationErrors(
                                                                            prev => ({
                                                                                ...prev,
                                                                                value: ''
                                                                            })
                                                                        );
                                                                }}
                                                                searchable
                                                                error={
                                                                    !!editingValidationErrors()
                                                                        .value
                                                                }
                                                                errorMessage={
                                                                    editingValidationErrors().value
                                                                }
                                                            />
                                                        </Show>
                                                        <Show when={field()?.type === 'folder'}>
                                                            <Select
                                                                size="sm"
                                                                options={hierarchicalFolders()}
                                                                value={String(editingValue() || '')}
                                                                onValueChange={val => {
                                                                    setEditingValue(Number(val));
                                                                    if (
                                                                        editingValidationErrors()
                                                                            .value
                                                                    )
                                                                        setEditingValidationErrors(
                                                                            prev => ({
                                                                                ...prev,
                                                                                value: ''
                                                                            })
                                                                        );
                                                                }}
                                                                searchable
                                                                error={
                                                                    !!editingValidationErrors()
                                                                        .value
                                                                }
                                                                errorMessage={
                                                                    editingValidationErrors().value
                                                                }
                                                            />
                                                        </Show>
                                                        <Show when={field()?.type === 'rating'}>
                                                            <Select
                                                                size="sm"
                                                                options={[0, 1, 2, 3, 4, 5].map(
                                                                    v => ({
                                                                        value: String(v),
                                                                        label: `${v} Stars`
                                                                    })
                                                                )}
                                                                value={String(
                                                                    editingValue() || '0'
                                                                )}
                                                                onValueChange={val => {
                                                                    setEditingValue(Number(val));
                                                                    if (
                                                                        editingValidationErrors()
                                                                            .value
                                                                    )
                                                                        setEditingValidationErrors(
                                                                            prev => ({
                                                                                ...prev,
                                                                                value: ''
                                                                            })
                                                                        );
                                                                }}
                                                                error={
                                                                    !!editingValidationErrors()
                                                                        .value
                                                                }
                                                                errorMessage={
                                                                    editingValidationErrors().value
                                                                }
                                                            />
                                                        </Show>
                                                        <Show when={field()?.type === 'select'}>
                                                            <Select
                                                                size="sm"
                                                                options={supportedFormats().flatMap(
                                                                    f =>
                                                                        f.extensions.map(ext => ({
                                                                            value: ext,
                                                                            label: ext.toUpperCase()
                                                                        }))
                                                                )}
                                                                value={editingValue() || ''}
                                                                onValueChange={val => {
                                                                    setEditingValue(val);
                                                                    if (
                                                                        editingValidationErrors()
                                                                            .value
                                                                    )
                                                                        setEditingValidationErrors(
                                                                            prev => ({
                                                                                ...prev,
                                                                                value: ''
                                                                            })
                                                                        );
                                                                }}
                                                                searchable
                                                                error={
                                                                    !!editingValidationErrors()
                                                                        .value
                                                                }
                                                                errorMessage={
                                                                    editingValidationErrors().value
                                                                }
                                                            />
                                                        </Show>
                                                    </div>
                                                }
                                            >
                                                {item.displayValue ||
                                                    computeDisplayValue(item, metadata)}
                                            </Show>
                                        </span>
                                        <Show
                                            when={!isEditing()}
                                            fallback={
                                                <Button
                                                    variant="ghost"
                                                    size="icon"
                                                    onClick={handleConfirmEdit}
                                                >
                                                    <Check size={16} />
                                                </Button>
                                            }
                                        >
                                            <Button
                                                variant="ghost"
                                                size="icon"
                                                onClick={() => handleStartEdit(item)}
                                            >
                                                <Pencil size={14} />
                                            </Button>
                                        </Show>
                                        <Button
                                            variant="ghost-destructive"
                                            size="icon"
                                            onClick={() => handleRemoveCriteria(item.id)}
                                        >
                                            <Trash2 size={14} />
                                        </Button>
                                    </div>
                                );
                            }}
                        </For>
                    </div>

                    <div class="match-mode-section">
                        <Tooltip content="Choose how to combine your criteria. 'All' requires every condition to be met, while 'Any' matches if at least one condition is met.">
                            <div class="section-title">
                                Match Mode <CircleQuestionMark size={12} />
                            </div>
                        </Tooltip>
                        <RadioGroup
                            value={matchMode()}
                            onValueChange={val => setMatchMode(val as LogicalOperator)}
                            orientation="horizontal"
                            class="match-radio-horizontal"
                        >
                            <RadioGroupItem value="or" label="Any (OR)" />
                            <RadioGroupItem value="and" label="All (AND)" />
                        </RadioGroup>
                    </div>
                </div>
            </div>
        </Modal>
    );
};
