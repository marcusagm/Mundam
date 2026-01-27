import { 
  Component, 
  For, 
  Show, 
  createSignal, 
  createEffect, 
  onCleanup,
  splitProps,
  createMemo,
} from "solid-js";
import { Portal } from "solid-js/web";
import { X } from "lucide-solid";
import { cn } from "../../lib/utils";
import { createId } from "../../lib/primitives/createId";
import "./tag-input.css";

export interface TagOption {
  id: string | number;
  label: string;
  color?: string;
}

export interface TagInputProps {
  /** Currently selected tags */
  value: TagOption[];
  /** Callback when tags change */
  onChange: (tags: TagOption[]) => void;
  /** Placeholder text when empty */
  placeholder?: string;
  /** Available tags for autocomplete suggestions */
  suggestions?: TagOption[];
  /** Callback to create a new tag */
  onCreate?: (name: string) => void;
  /** Whether the input is disabled */
  disabled?: boolean;
  /** Maximum number of tags allowed */
  max?: number;
  /** Additional CSS class */
  class?: string;
}

/**
 * TagInput component for managing a list of tags with autocomplete.
 * 
 * @example
 * const [tags, setTags] = createSignal<TagOption[]>([]);
 * 
 * <TagInput
 *   value={tags()}
 *   onChange={setTags}
 *   suggestions={[{ id: 1, label: "React" }, { id: 2, label: "Vue" }]}
 *   placeholder="Add tags..."
 * />
 */
export const TagInput: Component<TagInputProps> = (props) => {
  const [local] = splitProps(props, [
    "value",
    "onChange",
    "placeholder",
    "suggestions",
    "onCreate",
    "disabled",
    "max",
    "class",
  ]);

  const [inputValue, setInputValue] = createSignal("");
  const [showSuggestions, setShowSuggestions] = createSignal(false);
  const [highlightedIndex, setHighlightedIndex] = createSignal(-1);
  
  let inputContainerRef: HTMLDivElement | undefined;
  let inputRef: HTMLInputElement | undefined;
  let dropdownRef: HTMLUListElement | undefined;

  const id = createId("tag-input");
  const listboxId = `${id}-listbox`;

  const canAddMore = () => {
    if (local.max === undefined) return true;
    return local.value.length < local.max;
  };

  const filteredSuggestions = createMemo(() => {
    const input = inputValue().toLowerCase().trim();
    if (!input) return [];
    
    return (local.suggestions || [])
      .filter((t) => t.label.toLowerCase().includes(input))
      .filter((t) => !local.value.some((v) => v.id === t.id))
      .slice(0, 10); // Limit results
  });

  const updateDropdownPosition = () => {
    if (!inputContainerRef || !dropdownRef) return;
    
    const rect = inputContainerRef.getBoundingClientRect();
    dropdownRef.style.left = `${rect.left}px`;
    dropdownRef.style.top = `${rect.bottom + 4}px`;
    dropdownRef.style.width = `${rect.width}px`;
  };

  createEffect(() => {
    const visible = showSuggestions() && filteredSuggestions().length > 0;
    if (visible) {
      requestAnimationFrame(updateDropdownPosition);
    }
  });

  const handleKeyDown = (e: KeyboardEvent) => {
    const suggestions = filteredSuggestions();
    
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        if (suggestions.length > 0) {
          setHighlightedIndex((prev) => 
            prev < suggestions.length - 1 ? prev + 1 : 0
          );
        }
        break;
        
      case "ArrowUp":
        e.preventDefault();
        if (suggestions.length > 0) {
          setHighlightedIndex((prev) => 
            prev > 0 ? prev - 1 : suggestions.length - 1
          );
        }
        break;
        
      case "Enter":
        e.preventDefault();
        const val = inputValue().trim();
        if (!val || !canAddMore()) return;

        if (highlightedIndex() >= 0 && suggestions[highlightedIndex()]) {
          addTag(suggestions[highlightedIndex()]);
        } else {
          const exactMatch = suggestions.find(
            (t) => t.label.toLowerCase() === val.toLowerCase()
          );
          if (exactMatch) {
            addTag(exactMatch);
          } else if (local.onCreate) {
            local.onCreate(val);
            setInputValue("");
            setShowSuggestions(false);
          }
        }
        setHighlightedIndex(-1);
        break;
        
      case "Backspace":
        if (!inputValue() && local.value.length > 0) {
          const newVal = [...local.value];
          newVal.pop();
          local.onChange(newVal);
        }
        break;
        
      case "Escape":
        setShowSuggestions(false);
        setHighlightedIndex(-1);
        break;
    }
  };

  const addTag = (tag: TagOption) => {
    if (!canAddMore()) return;
    local.onChange([...local.value, tag]);
    setInputValue("");
    setShowSuggestions(false);
    setHighlightedIndex(-1);
    inputRef?.focus();
  };

  const removeTag = (id: string | number) => {
    local.onChange(local.value.filter((t) => t.id !== id));
    inputRef?.focus();
  };

  // Close on click outside
  createEffect(() => {
    if (!showSuggestions()) return;
    
    const handleClickOutside = (e: MouseEvent) => {
      const target = e.target as Node;
      if (
        !inputContainerRef?.contains(target) &&
        !dropdownRef?.contains(target)
      ) {
        setShowSuggestions(false);
      }
    };
    
    document.addEventListener("mousedown", handleClickOutside);
    onCleanup(() => document.removeEventListener("mousedown", handleClickOutside));
  });

  return (
    <div class={cn("ui-tag-input-wrapper", local.class)}>
      <div 
        ref={inputContainerRef}
        class={cn(
          "ui-tag-input-container",
          local.disabled && "ui-tag-input-disabled"
        )}
        onClick={() => inputRef?.focus()}
      >
        <For each={local.value}>
          {(tag) => (
            <span
              class="ui-tag-chip"
              style={tag.color ? { 
                "background-color": tag.color,
                "color": "white" 
              } : undefined}
            >
              <span class="ui-tag-chip-label">{tag.label}</span>
              <button
                type="button"
                class="ui-tag-chip-remove"
                onClick={(e) => {
                  e.stopPropagation();
                  removeTag(tag.id);
                }}
                aria-label={`Remove ${tag.label}`}
                disabled={local.disabled}
              >
                <X size={12} />
              </button>
            </span>
          )}
        </For>

        <input
          ref={inputRef}
          type="text"
          class="ui-tag-input"
          value={inputValue()}
          onInput={(e) => {
            setInputValue(e.currentTarget.value);
            setShowSuggestions(true);
            setHighlightedIndex(-1);
          }}
          onKeyDown={handleKeyDown}
          onFocus={() => setShowSuggestions(true)}
          placeholder={local.value.length === 0 ? local.placeholder : ""}
          disabled={local.disabled || !canAddMore()}
          aria-autocomplete="list"
          aria-controls={showSuggestions() ? listboxId : undefined}
          aria-activedescendant={
            highlightedIndex() >= 0 
              ? `${listboxId}-option-${highlightedIndex()}` 
              : undefined
          }
          role="combobox"
          aria-expanded={showSuggestions()}
          aria-haspopup="listbox"
        />
      </div>

      <Show when={showSuggestions() && filteredSuggestions().length > 0}>
        <Portal>
          <ul
            ref={dropdownRef}
            id={listboxId}
            role="listbox"
            class="ui-tag-suggestions"
            onMouseDown={(e) => e.preventDefault()}
          >
            <For each={filteredSuggestions()}>
              {(suggestion, index) => (
                <li
                  id={`${listboxId}-option-${index()}`}
                  role="option"
                  class={cn(
                    "ui-tag-suggestion-item",
                    highlightedIndex() === index() && "ui-tag-suggestion-highlighted"
                  )}
                  aria-selected={highlightedIndex() === index()}
                  onClick={() => addTag(suggestion)}
                  onMouseEnter={() => setHighlightedIndex(index())}
                >
                  {suggestion.label}
                </li>
              )}
            </For>
          </ul>
        </Portal>
      </Show>
    </div>
  );
};
