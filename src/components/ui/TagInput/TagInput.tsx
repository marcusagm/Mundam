import { Component, For, Show, splitProps } from 'solid-js';
import { cn } from '../../../lib/utils';
import { createId } from '../../../lib/primitives/createId';
import { createClickOutside } from '../../../lib/primitives';
import { useInputEvents } from '../Input/useInputEvents';
import { TagOption, TagInputProps } from './types';
import { TagChip } from './TagChip';
import { TagSuggestions } from './TagSuggestions';
import './tag-input.css';

// Hooks especializados para decomposição de complexidade
import { useTagInputState } from './hooks/useTagInputState';
import { useTagFloating } from './hooks/useTagFloating';
import { useTagNavigation } from './hooks/useTagNavigation';

/**
 * TagInput component for managing a list of tags with autocomplete support.
 * Integrates with the core input system for keyboard shortcut safety and accessibility.
 * Decomposed into specialized hooks for state, positioning, and navigation.
 *
 * @param props - Properties for the TagInput component.
 * @returns The rendered TagInput component.
 */
export const TagInput: Component<TagInputProps> = props => {
    // Separate specialized tag logic properties from standard attributes.
    const [localTagInputProperties] = splitProps(props, [
        'allowCustomValues',
        'ariaLabel',
        'value',
        'onChange',
        'placeholder',
        'suggestions',
        'onCreate',
        'disabled',
        'max',
        'class'
    ]);

    // Internal state and tag manipulation logic
    const {
        inputValue,
        setInputValue,
        showSuggestions,
        setShowSuggestions,
        highlightedSuggestionIndex,
        setHighlightedSuggestionIndex,
        filteredTagSuggestionsList,
        canAddMoreTags,
        addTag,
        removeTag
    } = useTagInputState(localTagInputProperties);

    let inputContainerReference: HTMLDivElement | undefined;
    let inputFieldReference: HTMLInputElement | undefined;
    let dropdownListReference: HTMLUListElement | undefined;

    const uniqueIdentifier = createId('tag-input');
    const suggestionListboxIdentifier = `${uniqueIdentifier}-listbox`;
    const suggestionScopeIdentifier = `tag-suggestions-${uniqueIdentifier}`;

    // Floating UI positioning logic for the suggestions dropdown
    const { suggestionDropdownCoordinates } = useTagFloating(
        () => inputContainerReference,
        () => dropdownListReference,
        () => showSuggestions() && filteredTagSuggestionsList().length > 0
    );

    // Keyboard navigation and isolation via core input system
    useTagNavigation({
        scopeIdentifier: suggestionScopeIdentifier,
        showSuggestions,
        suggestions: filteredTagSuggestionsList,
        highlightedIndex: highlightedSuggestionIndex,
        setHighlightedIndex: setHighlightedSuggestionIndex,
        inputValue,
        setInputValue,
        addTag,
        onCreate: (name: string) => localTagInputProperties.onCreate?.(name),
        canAddMoreTags,
        setShowSuggestions
    });

    // Custom core hook for shortcut safety and focus management.
    const {
        handleFocus,
        handleBlur,
        handleKeyDown: handleCoreKeyDown
    } = useInputEvents({
        onFocus: () => setShowSuggestions(true)
    });

    // Handle clicks outside the component to close suggestions.
    createClickOutside(
        () => [inputContainerReference, dropdownListReference].filter(Boolean) as HTMLElement[],
        () => {
            if (showSuggestions()) {
                setShowSuggestions(false);
            }
        }
    );

    /**
     * Handles local keyboard events not covered by scoped shortcuts (e.g., Backspace).
     *
     * @param event - The keyboard event object.
     */
    const handleLocalKeyDown = (event: KeyboardEvent) => {
        if (
            localTagInputProperties.allowCustomValues &&
            event.key === 'Enter' &&
            !event.isComposing
        ) {
            event.preventDefault();
            event.stopPropagation();
            commitCustomValues(inputValue());
            return;
        }
        if (
            event.key === 'Backspace' &&
            !inputValue() &&
            localTagInputProperties.value.length > 0
        ) {
            const updatedTagList = [...localTagInputProperties.value];
            updatedTagList.pop();
            localTagInputProperties.onChange(updatedTagList);
        }

        // Forward to core handler for shortcut safety of native editing keys.
        handleCoreKeyDown(event);
    };

    /**
     * Wrapper for addTag to ensure the input field regains focus.
     */
    const handleTagSelection = (tag: TagOption) => {
        addTag(tag);
        inputFieldReference?.focus();
    };

    /**
     * Wrapper for removeTag to ensure the input field regains focus.
     */
    const handleTagRemoval = (identifier: string | number) => {
        removeTag(identifier);
        inputFieldReference?.focus();
    };

    /** Commits one batch so paste and duplicate removal use a consistent selection. */
    const commitCustomValues = (text: string, keepLast = false): void => {
        const segments = text.split(',');
        const remainder = keepLast ? segments.pop() || '' : '';
        const selection = [...localTagInputProperties.value];
        for (const segment of segments) {
            const label = segment.trim();
            if (!label || selection.some(tag => tag.label.toLowerCase() === label.toLowerCase()))
                continue;
            if (
                localTagInputProperties.max !== undefined &&
                selection.length >= localTagInputProperties.max
            )
                break;
            selection.push({ id: label.toLowerCase(), label });
        }
        if (selection.length !== localTagInputProperties.value.length)
            localTagInputProperties.onChange(selection);
        setInputValue(remainder);
    };

    return (
        <div class={cn('ui-tag-input-wrapper', localTagInputProperties.class)}>
            <div
                ref={inputContainerReference}
                class={cn(
                    'ui-tag-input-container',
                    localTagInputProperties.disabled && 'ui-tag-input-disabled'
                )}
                onClick={() => inputFieldReference?.focus()}
            >
                <For each={localTagInputProperties.value}>
                    {tag => (
                        <TagChip
                            tag={tag}
                            onRemove={handleTagRemoval}
                            disabled={localTagInputProperties.disabled}
                        />
                    )}
                </For>

                <input
                    ref={inputFieldReference}
                    type="text"
                    class="ui-tag-input"
                    value={inputValue()}
                    onInput={event => {
                        if (
                            localTagInputProperties.allowCustomValues &&
                            !event.isComposing &&
                            event.currentTarget.value.includes(',')
                        ) {
                            commitCustomValues(event.currentTarget.value, true);
                        } else {
                            setInputValue(event.currentTarget.value);
                        }
                        setShowSuggestions(true);
                        setHighlightedSuggestionIndex(-1);
                    }}
                    onKeyDown={handleLocalKeyDown}
                    onFocus={handleFocus}
                    onBlur={event => {
                        if (localTagInputProperties.allowCustomValues)
                            commitCustomValues(inputValue());
                        handleBlur(event);
                    }}
                    aria-label={localTagInputProperties.ariaLabel}
                    placeholder={
                        localTagInputProperties.value.length === 0
                            ? localTagInputProperties.placeholder
                            : ''
                    }
                    disabled={localTagInputProperties.disabled || !canAddMoreTags()}
                    aria-autocomplete="list"
                    aria-controls={showSuggestions() ? suggestionListboxIdentifier : undefined}
                    aria-activedescendant={
                        highlightedSuggestionIndex() >= 0
                            ? `${suggestionListboxIdentifier}-option-${highlightedSuggestionIndex()}`
                            : undefined
                    }
                    role="combobox"
                    aria-expanded={showSuggestions()}
                    aria-haspopup="listbox"
                />
            </div>

            <Show when={showSuggestions() && filteredTagSuggestionsList().length > 0}>
                <TagSuggestions
                    suggestions={filteredTagSuggestionsList()}
                    highlightedIndex={highlightedSuggestionIndex()}
                    listboxId={suggestionListboxIdentifier}
                    position={suggestionDropdownCoordinates()}
                    onSelect={handleTagSelection}
                    onHighlight={setHighlightedSuggestionIndex}
                    ref={dropdownListReference}
                />
            </Show>
        </div>
    );
};
