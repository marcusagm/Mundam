import { createEffect, onCleanup, Accessor } from 'solid-js';
import { useInput, createShortcut, SCOPE_PRIORITIES } from '../../../../core/input';
import { TagOption } from '../types';

/**
 * Hook to manage keyboard navigation within the TagInput suggestions.
 * Integrates with the core input system for scoped shortcut safety and accessibility.
 *
 * @param options - Navigation and state functions.
 */
export const useTagNavigation = (options: {
    scopeIdentifier: string;
    showSuggestions: Accessor<boolean>;
    suggestions: Accessor<TagOption[]>;
    highlightedIndex: Accessor<number>;
    setHighlightedIndex: (index: number | ((prev: number) => number)) => void;
    inputValue: Accessor<string>;
    setInputValue: (value: string) => void;
    addTag: (tag: TagOption) => void;
    onCreate?: (name: string) => void;
    canAddMoreTags: Accessor<boolean>;
    setShowSuggestions: (show: boolean) => void;
}) => {
    const inputService = useInput();

    /**
     * Activates the specific 'tag-suggestions' scope for keyboard isolation.
     * Prevents global shortcuts from conflicting with suggestion navigation.
     */
    createEffect(() => {
        if (options.showSuggestions() && options.suggestions().length > 0) {
            inputService.pushScope(options.scopeIdentifier, SCOPE_PRIORITIES.modal + 10, true);
            onCleanup(() => inputService.popScope(options.scopeIdentifier));
        }
    });

    // Register scoped shortcuts for a seamless navigation experience.
    createShortcut({
        keys: 'ArrowDown',
        scope: options.scopeIdentifier,
        system: true,
        action: () => {
            const currentSuggestions = options.suggestions();
            if (currentSuggestions.length > 0) {
                options.setHighlightedIndex(previous =>
                    previous < currentSuggestions.length - 1 ? previous + 1 : 0
                );
            }
        },
        ignoreInputs: false,
        preventDefault: true
    });

    createShortcut({
        keys: 'ArrowUp',
        scope: options.scopeIdentifier,
        system: true,
        action: () => {
            const currentSuggestions = options.suggestions();
            if (currentSuggestions.length > 0) {
                options.setHighlightedIndex(previous =>
                    previous > 0 ? previous - 1 : currentSuggestions.length - 1
                );
            }
        },
        ignoreInputs: false,
        preventDefault: true
    });

    createShortcut({
        keys: 'Enter',
        scope: options.scopeIdentifier,
        system: true,
        action: () => {
            const currentSuggestions = options.suggestions();
            const trimmedInput = options.inputValue().trim();
            if (!trimmedInput || !options.canAddMoreTags()) {
                return;
            }

            const highlightedTagIndex = options.highlightedIndex();
            const highlightedTag = currentSuggestions[highlightedTagIndex];

            if (highlightedTagIndex >= 0 && highlightedTag) {
                options.addTag(highlightedTag);
            } else {
                const exactMatch = currentSuggestions.find(
                    tag => tag.label.toLowerCase() === trimmedInput.toLowerCase()
                );

                if (exactMatch) {
                    options.addTag(exactMatch);
                } else if (options.onCreate) {
                    options.onCreate(trimmedInput);
                    options.setInputValue('');
                    options.setShowSuggestions(false);
                }
            }
            options.setHighlightedIndex(-1);
        },
        ignoreInputs: false,
        preventDefault: true
    });

    createShortcut({
        keys: 'Escape',
        scope: options.scopeIdentifier,
        system: true,
        action: () => {
            options.setShowSuggestions(false);
            options.setHighlightedIndex(-1);
        },
        ignoreInputs: false,
        preventDefault: true
    });
};
