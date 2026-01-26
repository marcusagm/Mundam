import { Component, For, createSignal, Show } from "solid-js";
import { X } from "lucide-solid";
import "./tag-input.css";

export interface TagOption {
    id: string | number;
    label: string;
    color?: string;
}

interface TagInputProps {
    value: TagOption[];
    onChange: (tags: TagOption[]) => void;
    placeholder?: string;
    suggestions?: TagOption[]; // All available tags for autocomplete
    onCreate?: (name: string) => void; // Optional: create new tag
}

export const TagInput: Component<TagInputProps> = (props) => {
    const [inputValue, setInputValue] = createSignal("");
    const [showSuggestions, setShowSuggestions] = createSignal(false);
    
    // Filter suggestions based on input
    const filteredSuggestions = () => {
        const input = inputValue().toLowerCase();
        if (!input) return [];
        return (props.suggestions || [])
            .filter(t => t.label.toLowerCase().includes(input))
            .filter(t => !props.value.some(v => v.id === t.id)); // Exclude already selected
    };

    const handleKeyDown = (e: KeyboardEvent) => {
        if (e.key === "Enter") {
            e.preventDefault();
            const val = inputValue().trim();
            if (!val) return;
            
            const relevant = filteredSuggestions();
            // If there's an exact match or user selected one, use it. 
            // For now, if there's one relevant suggestion that matches exactly, take it?
            // Or just check if onCreate is passed.
            
            const exactMatch = relevant.find(t => t.label.toLowerCase() === val.toLowerCase());
            
            if (exactMatch) {
                addTag(exactMatch);
            } else if (relevant.length > 0) {
                 // Should we auto-select first? Maybe better to require explicit selection or match.
                 // Let's rely on onCreate if no match found.
                 if (props.onCreate) {
                    props.onCreate(val);
                    setInputValue("");
                    setShowSuggestions(false);
                 }
            } else if (props.onCreate) {
                props.onCreate(val);
                setInputValue("");
                setShowSuggestions(false);
            }
        } else if (e.key === "Backspace" && !inputValue() && props.value.length > 0) {
            // Remove last tag
            const newVal = [...props.value];
            newVal.pop();
            props.onChange(newVal);
        }
    };

    const addTag = (tag: TagOption) => {
        props.onChange([...props.value, tag]);
        setInputValue("");
        setShowSuggestions(false);
    };

    const removeTag = (id: string | number) => {
        props.onChange(props.value.filter(t => t.id !== id));
    };

    return (
        <div class="tag-input-container">
            <div class="tag-input-content">
                <For each={props.value}>
                    {(tag) => (
                        <div 
                            class="tag-chip" 
                            style={tag.color ? { "background-color": tag.color, "color": "white" } : {}}
                        >
                            <span>{tag.label}</span>
                            <button onClick={() => removeTag(tag.id)} class="tag-remove-btn">
                                <X size={12} />
                            </button>
                        </div>
                    )}
                </For>
                <input
                    type="text"
                    value={inputValue()}
                    onInput={(e) => {
                        setInputValue(e.currentTarget.value);
                        setShowSuggestions(true);
                    }}
                    onKeyDown={handleKeyDown}
                    onBlur={() => setTimeout(() => setShowSuggestions(false), 200)}
                    onFocus={() => setShowSuggestions(true)}
                    placeholder={props.value.length === 0 ? props.placeholder : ""}
                    class="tag-input-field"
                />
            </div>
            
            <Show when={showSuggestions() && inputValue() && filteredSuggestions().length > 0}>
                <div class="tag-suggestions-dropdown">
                    <For each={filteredSuggestions()}>
                        {(suggestion) => (
                            <div 
                                class="tag-suggestion-item" 
                                onMouseDown={(e) => { e.preventDefault(); addTag(suggestion); }}
                            >
                                {suggestion.label}
                            </div>
                        )}
                    </For>
                </div>
            </Show>
        </div>
    );
};
