/**
 * TagInput component
 *
 * @module TagInput
 * @description
 * The TagInput component is a specialized Input component that allows the user to enter tags.
 * It provides controls for adding/removing tags and filtering suggestions.
 * Enable `allowCustomValues` to commit free-text chips with commas, Enter or blur.
 * This mode trims whitespace and deduplicates labels without changing taxonomy inputs.
 *
 * @example
 * ```tsx
 * import { TagInput } from '@/components/ui';
 *
 * <TagInput
 *   value={[{ id: 'nature', label: 'Nature' }]}
 *   onChange={handleChange}
 *   placeholder="Enter tags"
 *   suggestions={[{ id: 'nature', label: 'Nature' }]}
 *   onCreate={handleCreate}
 *   disabled={false}
 *   max={10}
 *   class="tag-input"
 * />
 * ```
 */
export * from './TagInput';
export * from './TagChip';
export * from './TagSuggestions';
export * from './types';
