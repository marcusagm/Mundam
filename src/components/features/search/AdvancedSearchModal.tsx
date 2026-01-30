import { Component, createSignal, Show, For, createMemo, createEffect } from "solid-js";
import fileFormats from "../../../core/constants/fileFormats.json";
import { 
    Plus, 
    Trash2, 
    Search, 
    Save, 
    HelpCircle, 
    Info, 
    ChevronRight,
} from "lucide-solid";
import { Modal } from "../../ui/Modal";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { RadioGroup, RadioGroupItem } from "../../ui/RadioGroup";
import { useFilters, useMetadata } from "../../../core/hooks";
import { SearchCriterion, SearchGroup, LogicalOperator } from "../../../core/store/filterStore";
import { createId } from "../../../lib/primitives/createId";
import { MaskedInput } from "../../ui/MaskedInput";
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
    createMemo(() => {
        const field = selectedField();
        if (field) {
            const defaultOp = OPERATORS_FOR_TYPE[field.type]?.[0]?.value;
            setCurrentOperator(defaultOp || "");
            setCurrentValue(null);
            setCurrentValue2(null);
        }
    });

    const handleAddCriteria = () => {
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
                <div class="match-section">
                    <RadioGroup 
                        value={matchMode()} 
                        onValueChange={(val) => setMatchMode(val as LogicalOperator)}
                        class="match-radio"
                    >
                        <RadioGroupItem value="or" label="Any" />
                        <RadioGroupItem value="and" label="All" />
                    </RadioGroup>
                <div style={{ flex: 1 }} />
                <Button variant="outline" onClick={handleReset}>Reset</Button>
                
                <Show when={criteria().length > 0}>
                    <Button variant="outline" onClick={handleSaveSmartFolder} disabled={!smartFolderName()}>
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
                    <div class="section-title">
                        Criteria Builder <HelpCircle size={12} />
                    </div>
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
                                    onInput={(e) => setCurrentValue(e.currentTarget.value)} 
                                    placeholder="Value..."
                                />
                            </Show>
                            <Show when={selectedField()?.type === 'number'}>
                                <div class="number-input-group">
                                    <Input 
                                        type="number" 
                                        size="sm"
                                        value={currentValue() || ""} 
                                        onInput={(e) => setCurrentValue(Number(e.currentTarget.value))} 
                                        placeholder={currentOperator() === 'between' ? "From..." : "Value..."}
                                    />
                                    <Show when={currentOperator() === 'between'}>
                                        <span class="range-separator">to</span>
                                        <Input 
                                            type="number" 
                                            size="sm"
                                            value={currentValue2() || ""} 
                                            onInput={(e) => setCurrentValue2(Number(e.currentTarget.value))} 
                                            placeholder="To..."
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
                                        onInput={setCurrentValue} 
                                        placeholder={currentOperator() === 'between' ? "From DD/MM/YYYY" : "DD/MM/YYYY"}
                                    />
                                    <Show when={currentOperator() === 'between'}>
                                        <span class="range-separator">to</span>
                                        <MaskedInput 
                                            size="sm"
                                            mask="99/99/9999"
                                            value={currentValue2() || ""} 
                                            onInput={setCurrentValue2} 
                                            placeholder="To DD/MM/YYYY"
                                        />
                                    </Show>
                                </div>
                            </Show>
                            <Show when={selectedField()?.type === 'tags'}>
                                <div class="tag-select-placeholder">
                                    <Select 
                                        options={hierarchicalTags()}
                                        value={String(currentValue() || "")}
                                        onValueChange={(val) => setCurrentValue(val)}
                                        placeholder="Select Tag..."
                                        searchable
                                    />
                                </div>
                            </Show>
                             <Show when={selectedField()?.type === 'folder'}>
                                <div class="folder-select-placeholder">
                                    <Select 
                                        options={hierarchicalFolders()}
                                        value={String(currentValue() || "")}
                                        onValueChange={(val) => setCurrentValue(Number(val))}
                                        placeholder="Select Folder..."
                                        searchable
                                    />
                                </div>
                            </Show>
                            <Show when={selectedField()?.type === 'rating'}>
                                <Select 
                                    options={[0,1,2,3,4,5].map(v => ({ value: String(v), label: `${v} Stars` }))}
                                    value={String(currentValue() || "0")}
                                    onValueChange={(val) => setCurrentValue(Number(val))}
                                />
                            </Show>
                            <Show when={selectedField()?.type === 'select'}>
                                <Select 
                                    options={(fileFormats as FileFormat[]).map(f => ({ value: f.extension, label: `${f.extension.toUpperCase()} - ${f.name}` }))}
                                    value={currentValue() || ""}
                                    onValueChange={setCurrentValue}
                                    searchable
                                />
                            </Show>
                        </div>

                        <Button variant="ghost" size="icon" onClick={handleAddCriteria} disabled={currentValue() === null || currentValue() === ""}>
                            <Plus size={18} />
                        </Button>
                    </div>
                </div>

                <div class="query-editor-section">
                    <div class="section-title">
                        Query Editor <HelpCircle size={12} />
                    </div>
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
                            {(item, index) => (
                                <div class="criterion-item">
                                    <span class="criterion-index">{index() + 1}</span>
                                    <span class="criterion-field">
                                        {SEARCH_FIELDS.find(f => f.value === item.key)?.label || item.key}
                                    </span>
                                    <span class="criterion-operator">
                                        {availableOperators().find(o => o.value === item.operator)?.label || item.operator}
                                    </span>
                                    <span class="criterion-value">
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
                                    </span>
                                    <Button variant="ghost" size="icon" class="edit-criterion-btn">
                                        {/* Edit logic omitted for brevity in first version */}
                                        <ChevronRight size={14} />
                                    </Button>
                                    <Button variant="ghost" size="icon" class="remove-criterion-btn" onClick={() => handleRemoveCriteria(item.id)}>
                                        <Trash2 size={14} color="var(--danger-color)" />
                                    </Button>
                                </div>
                            )}
                        </For>
                    </div>
                </div>
            </div>
        </Modal>
    );
};
