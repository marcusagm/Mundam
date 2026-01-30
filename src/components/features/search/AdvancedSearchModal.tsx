import { Component, createSignal, Show, For, createMemo, createEffect } from "solid-js";
import fileFormats from "../../../core/constants/fileFormats.json";
import { 
    Plus, 
    Trash2, 
    Search, 
    Save, 
    CircleQuestionMark, 
    Info, 
    Pencil,
    Check,
} from "lucide-solid";
import { Modal } from "../../ui/Modal";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { RadioGroup, RadioGroupItem } from "../../ui/RadioGroup";
import { Tooltip } from "../../ui/Tooltip";
import { useFilters, useMetadata } from "../../../core/hooks";
import { SearchCriterion, SearchGroup, LogicalOperator } from "../../../core/store/filterStore";
import { createId } from "../../../lib/primitives/createId";
import { MaskedInput } from "../../ui/MaskedInput";
import { cn } from "../../../lib/utils";
import "./advanced-search-modal.css";

interface FileFormat {
    extension: string;
    name: string;
    description: string;
    tier: number;
}

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
    { value: "tags", label: "Tags", type: "tags" },
    { value: "filename", label: "Filename", type: "text" },
    { value: "format", label: "Format", type: "select" },
    { value: "size", label: "File size", type: "number" },
    { value: "width", label: "Width", type: "number" },
    { value: "height", label: "Height", type: "number" },
    { value: "added_at", label: "Date added", type: "date" },
    { value: "created_at", label: "Date creation", type: "date" },
    { value: "modified_at", label: "Date modified", type: "date" },
    { value: "rating", label: "Rating", type: "rating" },
    { value: "notes", label: "Notes", type: "text" },
    { value: "folder", label: "Folder", type: "folder" },
];

const OPERATORS_FOR_TYPE: Record<string, { value: string, label: string }[]> = {
    text: [
        { value: "contains", label: "Contains" },
        { value: "not_contains", label: "Not Contains" },
        { value: "equals", label: "Equals" },
        { value: "starts_with", label: "Starts With" },
        { value: "ends_with", label: "Ends With" },
    ],
    number: [
        { value: "gt", label: "Greater than" },
        { value: "lt", label: "Less than" },
        { value: "eq", label: "Equals" },
        { value: "between", label: "Between" },
    ],
    date: [
        { value: "before", label: "Before" },
        { value: "after", label: "After" },
        { value: "on", label: "On" },
        { value: "between", label: "Between" },
    ],
    select: [
        { value: "eq", label: "Equals" },
        { value: "neq", label: "Not Equals" },
    ],
    tags: [
        { value: "contains", label: "Contains" },
        { value: "not_contains", label: "Not Contains" },
    ],
    folder: [
        { value: "is", label: "Is" },
        { value: "in", label: "Is inside (recursive)" },
    ],
    rating: [
        { value: "eq", label: "Equals" },
        { value: "gte", label: "Greater than or equal" },
        { value: "lte", label: "Less than or equal" },
    ]
};

export const AdvancedSearchModal: Component<AdvancedSearchModalProps> = (props) => {
    const filters = useFilters();
    const metadata = useMetadata();

    const [smartFolderName, setSmartFolderName] = createSignal(props.initialName || "");
    const [currentKey, setCurrentKey] = createSignal("tags");
    const [currentOperator, setCurrentOperator] = createSignal("contains");
    const [currentValue, setCurrentValue] = createSignal<any>(null);
    const [currentUnit, setCurrentUnit] = createSignal("MB");
    const [currentValue2, setCurrentValue2] = createSignal<any>(null);
    const [criteria, setCriteria] = createSignal<SearchCriterion[]>([]);
    const [matchMode, setMatchMode] = createSignal<LogicalOperator>("and");
    const [validationErrors, setValidationErrors] = createSignal<Record<string, string>>({});
    
    // Editing state
    const [editingId, setEditingId] = createSignal<string | null>(null);
    const [editingValue, setEditingValue] = createSignal<any>(null);
    const [editingValue2, setEditingValue2] = createSignal<any>(null);

    const SIZE_UNITS = [
        { value: "1", label: "Bytes" },
        { value: "1024", label: "KB" },
        { value: "1048576", label: "MB" },
        { value: "1073741824", label: "GB" }
    ];

    // Initialize from props when opening
    createEffect(() => {
        if (props.isOpen) {
            setSmartFolderName(props.initialName || "");
            if (props.initialQuery) {
                setMatchMode(props.initialQuery.logicalOperator);
                // Filter out nested groups for now as builder doesn't support them yet
                const initialCriteria = props.initialQuery.items.filter(item => !('items' in item)) as SearchCriterion[];
                setCriteria(initialCriteria);
            } else {
                setCriteria([]);
                setMatchMode("and");
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
            setCurrentOperator(defaultOp || "");
            setCurrentValue(null);
            setCurrentValue2(null);
            setValidationErrors({});
        }
    });

    const validateCurrent = () => {
        const errors: Record<string, string> = {};
        const field = selectedField();
        const op = currentOperator();
        const val = currentValue();
        const val2 = currentValue2();

        if (val === null || val === "") {
            errors.value = "Value is required";
        }

        if (op === 'between' && (val2 === null || val2 === "")) {
            errors.value2 = "End value is required";
        }

        if (field?.type === 'date') {
            const dateRegex = /^\d{2}\/\d{2}\/\d{4}$/;
            if (val && !dateRegex.test(val)) errors.value = "Invalid date format";
            if (op === 'between' && val2 && !dateRegex.test(val2)) errors.value2 = "Invalid date format";
        }

        setValidationErrors(errors);
        return Object.keys(errors).length === 0;
    };

    const handleStartEdit = (item: SearchCriterion) => {
        setEditingId(item.id);
        if (Array.isArray(item.value)) {
            if (item.key === 'size') {
                // Convert back to MB for editing display
                setEditingValue(Number(item.value[0]) / 1024 / 1024);
                setEditingValue2(Number(item.value[1]) / 1024 / 1024);
            } else {
                setEditingValue(item.value[0]);
                setEditingValue2(item.value[1]);
            }
        } else {
            if (item.key === 'size') {
                setEditingValue(Number(item.value) / 1024 / 1024);
            } else {
                setEditingValue(item.value);
            }
            setEditingValue2(null);
        }
    };

    const handleConfirmEdit = () => {
        const id = editingId();
        if (!id) return;

        setCriteria(prev => prev.map(c => {
            if (c.id === id) {
                let finalValue = editingValue();
                
                // Handle size conversion (assume MB for edit simplification)
                if (c.key === 'size') {
                    if (Array.isArray(c.value) || c.operator === 'between') {
                        finalValue = [
                            Math.round(Number(editingValue()) * 1024 * 1024),
                            Math.round(Number(editingValue2()) * 1024 * 1024)
                        ];
                    } else {
                        finalValue = Math.round(Number(editingValue()) * 1024 * 1024);
                    }
                } else if (c.operator === 'between') {
                    finalValue = [editingValue(), editingValue2()];
                }
                
                return { ...c, value: finalValue };
            }
            return c;
        }));
        setEditingId(null);
    };

    const handleAddCriteria = () => {
        if (!validateCurrent()) return;

        let finalValue = currentValue();
        
        // Handle Unit conversion for size
        if (currentKey() === 'size' && finalValue !== null) {
            const multiplier = Number(currentUnit());
            if (currentOperator() === 'between') {
                finalValue = [
                    Math.round(Number(finalValue) * multiplier),
                    Math.round(Number(currentValue2()) * multiplier)
                ];
            } else {
                finalValue = Math.round(Number(finalValue) * multiplier);
            }
        } else if (currentOperator() === 'between') {
            finalValue = [currentValue(), currentValue2()];
        }

        const newCriterion: SearchCriterion = {
            id: createId("criterion"),
            key: currentKey(),
            operator: currentOperator(),
            value: finalValue,
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
        setMatchMode("and");
    };

    const handleSearch = () => {
        const searchGroup: SearchGroup = {
            id: createId("group"),
            logicalOperator: matchMode(),
            items: criteria(),
        };
        filters.setAdvancedSearch(searchGroup);
        props.onClose();
    };

    const handleSaveSmartFolder = () => {
        if (!smartFolderName()) return;
        const searchGroup: SearchGroup = {
            id: createId("group"),
            logicalOperator: matchMode(),
            items: criteria(),
        };
        props.onSave?.(smartFolderName(), searchGroup, props.initialId);
        props.onClose();
    };

    const hierarchicalTags = createMemo(() => {
        const indent = (tags: any[], parentId: number | null = null, depth = 0): { value: string, label: string }[] => {
            return tags
                .filter(t => (t.parent_id === parentId) || (parentId === null && !t.parent_id))
                .flatMap(t => [
                    { value: String(t.id), label: `${"\u00A0".repeat(depth * 3)}${t.name}` },
                    ...indent(tags, t.id, depth + 1)
                ]);
        };
        return indent(metadata.tags);
    });

    const hierarchicalFolders = createMemo(() => {
        const indent = (folders: any[], parentId: number | null = null, depth = 0): { value: string, label: string }[] => {
            return folders
                .filter(f => f.parent_id === parentId)
                .flatMap(f => [
                    { value: String(f.id), label: `${"\u00A0".repeat(depth * 3)}${f.name}` },
                    ...indent(folders, f.id, depth + 1)
                ]);
        };
        return indent(metadata.locations);
    });

    return (
        <Modal
            isOpen={props.isOpen}
            onClose={props.onClose}
            title={props.isSmartFolderMode ? "Smart Folder Configuration" : "Advanced Search"}
            size="lg"
            footer={
                <div class="modal-footer-content">
                    <Button variant="secondary" onClick={handleReset}>Reset</Button>
                    <div style={{ flex: 1 }} />
                    
                    <Show when={criteria().length > 0}>
                        <Button variant="secondary" onClick={handleSaveSmartFolder} disabled={!smartFolderName()}>
                            <Save size={16} class="mr-2" />
                            Save Smart Folder
                        </Button>
                    </Show>

                    <Button variant="primary" onClick={handleSearch} disabled={criteria().length === 0}>
                        <Search size={16} class="mr-2" />
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
                        onInput={(e) => setSmartFolderName(e.currentTarget.value)} 
                        placeholder="e.g. Pictures from Tokyo"
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
                                    size="sm"
                                    value={currentValue() || ""} 
                                    onInput={(e) => {
                                        setCurrentValue(e.currentTarget.value);
                                        if (validationErrors().value) setValidationErrors(prev => ({ ...prev, value: "" }));
                                    }} 
                                    placeholder="Value..."
                                    error={!!validationErrors().value}
                                    errorMessage={validationErrors().value}
                                />
                            </Show>
                            <Show when={selectedField()?.type === 'number'}>
                                <div class="number-input-group">
                                    <Input 
                                        type="number" 
                                        size="sm"
                                        value={currentValue() || ""} 
                                        onInput={(e) => {
                                            setCurrentValue(Number(e.currentTarget.value));
                                            if (validationErrors().value) setValidationErrors(prev => ({ ...prev, value: "" }));
                                        }} 
                                        placeholder={currentOperator() === 'between' ? "From..." : "Value..."}
                                        error={!!validationErrors().value}
                                        errorMessage={validationErrors().value}
                                    />
                                    <Show when={currentOperator() === 'between'}>
                                        <span class="range-separator">to</span>
                                        <Input 
                                            type="number" 
                                            size="sm"
                                            value={currentValue2() || ""} 
                                            onInput={(e) => {
                                                setCurrentValue2(Number(e.currentTarget.value));
                                                if (validationErrors().value2) setValidationErrors(prev => ({ ...prev, value2: "" }));
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
                                            onValueChange={setCurrentUnit}
                                        />
                                    </Show>
                                </div>
                            </Show>
                             <Show when={selectedField()?.type === 'date'}>
                                <div class="date-input-group">
                                    <MaskedInput 
                                        size="sm"
                                        mask="99/99/9999"
                                        value={currentValue() || ""} 
                                        onInput={(val) => {
                                            setCurrentValue(val);
                                            if (validationErrors().value) setValidationErrors(prev => ({ ...prev, value: "" }));
                                        }} 
                                        placeholder={currentOperator() === 'between' ? "From DD/MM/YYYY" : "DD/MM/YYYY"}
                                        error={!!validationErrors().value}
                                        errorMessage={validationErrors().value}
                                    />
                                    <Show when={currentOperator() === 'between'}>
                                        <span class="range-separator">to</span>
                                        <MaskedInput 
                                            size="sm"
                                            mask="99/99/9999"
                                            value={currentValue2() || ""} 
                                            onInput={(val) => {
                                                setCurrentValue2(val);
                                                if (validationErrors().value2) setValidationErrors(prev => ({ ...prev, value2: "" }));
                                            }} 
                                            placeholder="To DD/MM/YYYY"
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
                                        value={String(currentValue() || "")}
                                        onValueChange={(val) => {
                                            setCurrentValue(val);
                                            if (validationErrors().value) setValidationErrors(prev => ({ ...prev, value: "" }));
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
                                        value={String(currentValue() || "")}
                                        onValueChange={(val) => {
                                            setCurrentValue(Number(val));
                                            if (validationErrors().value) setValidationErrors(prev => ({ ...prev, value: "" }));
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
                                    options={[0,1,2,3,4,5].map(v => ({ value: String(v), label: `${v} Stars` }))}
                                    value={String(currentValue() || "0")}
                                    onValueChange={(val) => {
                                        setCurrentValue(Number(val));
                                        if (validationErrors().value) setValidationErrors(prev => ({ ...prev, value: "" }));
                                    }}
                                    error={!!validationErrors().value}
                                    errorMessage={validationErrors().value}
                                />
                            </Show>
                            <Show when={selectedField()?.type === 'select'}>
                                <Select 
                                    options={(fileFormats as FileFormat[]).map(f => ({ value: f.extension, label: `${f.extension.toUpperCase()} - ${f.name}` }))}
                                    value={currentValue() || ""}
                                    onValueChange={(val) => {
                                        setCurrentValue(val);
                                        if (validationErrors().value) setValidationErrors(prev => ({ ...prev, value: "" }));
                                    }}
                                    searchable
                                    error={!!validationErrors().value}
                                    errorMessage={validationErrors().value}
                                />
                            </Show>
                        </div>

                        <div class="builder-actions">
                            <Button variant="ghost" size="icon" onClick={handleAddCriteria} class="add-button">
                                <Plus size={18} />
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
                                    <strong>Empty Query</strong><br/>
                                    Your query is currently empty. Create a criteria above to enable search.
                                </div>
                            </div>
                        </Show>
                        <For each={criteria()}>
                            {(item, index) => {
                                const field = createMemo(() => SEARCH_FIELDS.find(f => f.value === item.key));
                                const isEditing = () => editingId() === item.id;

                                return (
                                    <div class={cn("criterion-item", isEditing() && "editing")}>
                                        <span class="criterion-index">{index() + 1}</span>
                                        <span class="criterion-field">
                                            {field()?.label || item.key}
                                        </span>
                                        <span class="criterion-operator">
                                            {OPERATORS_FOR_TYPE[field()?.type || '']?.find(o => o.value === item.operator)?.label || item.operator}
                                        </span>
                                        <span class="criterion-value">
                                            <Show when={!isEditing()} fallback={
                                                <div class="edit-inputs">
                                                    <Show when={field()?.type === 'text'}>
                                                        <Input size="sm" value={editingValue() || ""} onInput={(e) => setEditingValue(e.currentTarget.value)} />
                                                    </Show>
                                                    <Show when={field()?.type === 'number' || item.key === 'size'}>
                                                        <div class="horizontal-inputs">
                                                            <Input type="number" size="sm" value={editingValue() || ""} onInput={(e) => setEditingValue(Number(e.currentTarget.value))} />
                                                            <Show when={item.operator === 'between'}>
                                                                <span>to</span>
                                                                <Input type="number" size="sm" value={editingValue2() || ""} onInput={(e) => setEditingValue2(Number(e.currentTarget.value))} />
                                                            </Show>
                                                            <Show when={item.key === 'size'}>
                                                                <span class="unit-text">MB</span>
                                                            </Show>
                                                        </div>
                                                    </Show>
                                                    <Show when={field()?.type === 'date'}>
                                                        <div class="horizontal-inputs">
                                                            <MaskedInput size="sm" mask="99/99/9999" value={editingValue() || ""} onInput={setEditingValue} />
                                                            <Show when={item.operator === 'between'}>
                                                                <span>to</span>
                                                                <MaskedInput size="sm" mask="99/99/9999" value={editingValue2() || ""} onInput={setEditingValue2} />
                                                            </Show>
                                                        </div>
                                                    </Show>
                                                    <Show when={field()?.type === 'tags'}>
                                                        <Select options={hierarchicalTags()} value={String(editingValue() || "")} onValueChange={setEditingValue} searchable />
                                                    </Show>
                                                    <Show when={field()?.type === 'folder'}>
                                                        <Select options={hierarchicalFolders()} value={String(editingValue() || "")} onValueChange={(val) => setEditingValue(Number(val))} searchable />
                                                    </Show>
                                                    <Show when={field()?.type === 'rating'}>
                                                        <Select options={[0,1,2,3,4,5].map(v => ({ value: String(v), label: `${v} Stars` }))} value={String(editingValue() || "0")} onValueChange={(val) => setEditingValue(Number(val))} />
                                                    </Show>
                                                    <Show when={field()?.type === 'select'}>
                                                        <Select options={(fileFormats as FileFormat[]).map(f => ({ value: f.extension, label: f.extension.toUpperCase() }))} value={editingValue() || ""} onValueChange={setEditingValue} searchable />
                                                    </Show>
                                                </div>
                                            }>
                                                {Array.isArray(item.value) 
                                                    ? item.key === 'size'
                                                        ? `${(Number(item.value[0]) / 1024 / 1024).toFixed(2)} MB to ${(Number(item.value[1]) / 1024 / 1024).toFixed(2)} MB`
                                                        : `${item.value[0]} to ${item.value[1]}` 
                                                    : item.key === 'folder' 
                                                        ? metadata.locations.find(l => l.id === item.value)?.name || item.value
                                                        : item.key === 'tags'
                                                            ? metadata.tags.find(t => t.id === Number(item.value))?.name || item.value
                                                            : item.key === 'size'
                                                                ? `${(Number(item.value) / 1024 / 1024).toFixed(2)} MB`
                                                                : String(item.value)
                                                }
                                            </Show>
                                        </span>
                                        <Show when={!isEditing()} fallback={
                                            <Button variant="ghost" size="icon" onClick={handleConfirmEdit}>
                                                <Check size={16} />
                                            </Button>
                                        }>
                                            <Button variant="ghost" size="icon" onClick={() => handleStartEdit(item)}>
                                                <Pencil size={14} />
                                            </Button>
                                        </Show>
                                        <Button variant="ghost-destructive" size="icon" onClick={() => handleRemoveCriteria(item.id)}>
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
                            onValueChange={(val) => setMatchMode(val as LogicalOperator)}
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
