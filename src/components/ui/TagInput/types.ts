/**
 * Represents a single tag option.
 */
export interface TagOption {
    /** Unique identifier for the tag */
    id: string | number;
    /** Display text for the tag */
    label: string;
    /** Optional background color for the tag chip */
    color?: string;
}

/**
 * Properties for the TagInput component.
 */
export interface TagInputProps {
    /** Currently selected tags */
    value: TagOption[];

    /** Opt-in free-text chips committed with comma, Enter or blur. */
    allowCustomValues?: boolean;

    /** Accessible name for the editable field. */
    ariaLabel?: string;

    /** Callback executed when the tag list changes */
    onChange: (tags: TagOption[]) => void;

    /** Placeholder text displayed when the input is empty */
    placeholder?: string;

    /** Available tags for autocomplete suggestions */
    suggestions?: TagOption[];

    /** Callback to create a new tag if it doesn't exist in suggestions */
    onCreate?: (name: string) => void;

    /** Whether the input is disabled */
    disabled?: boolean;

    /** Maximum number of tags allowed */
    max?: number;

    /** Additional CSS class for the wrapper */
    class?: string;
}
