import { cleanup, fireEvent, render, screen } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createSignal } from 'solid-js';
import { InputProvider } from '../../../core/input/context';
import { TagInput } from './TagInput';
import { type TagOption } from './types';

afterEach(cleanup);

function renderKeywords() {
    render(() => {
        const [value, setValue] = createSignal<TagOption[]>([]);
        return (
            <InputProvider>
                <TagInput
                    ariaLabel="Keywords"
                    value={value()}
                    onChange={setValue}
                    allowCustomValues
                />
            </InputProvider>
        );
    });
    return screen.getByRole('combobox', { name: 'Keywords' });
}

describe('TagInput custom values', () => {
    it('keeps a pasted trailing draft, commits on blur and removes chips', () => {
        const input = renderKeywords();
        fireEvent.input(input, { target: { value: 'nature, concept art' } });
        expect(screen.getByText('nature')).toBeTruthy();
        expect((input as HTMLInputElement).value).toBe(' concept art');
        fireEvent.blur(input);
        expect(screen.getByText('concept art')).toBeTruthy();
        fireEvent.click(screen.getByRole('button', { name: 'Remove nature' }));
        expect(screen.queryByText('nature')).toBeNull();
    });

    it('keeps default taxonomy input unchanged when a comma is entered', () => {
        const onChange = vi.fn();
        render(() => (
            <InputProvider>
                <TagInput ariaLabel="Tags" value={[]} onChange={onChange} />
            </InputProvider>
        ));
        const input = screen.getByRole('combobox', { name: 'Tags' });
        fireEvent.input(input, { target: { value: 'parent, child' } });
        expect(onChange).not.toHaveBeenCalled();
        expect((input as HTMLInputElement).value).toBe('parent, child');
    });

    it('does not commit Enter during IME composition', () => {
        const input = renderKeywords();
        fireEvent.input(input, { target: { value: 'nature' } });
        fireEvent.keyDown(input, { key: 'Enter', isComposing: true });
        expect(screen.queryByText('nature')).toBeNull();
        fireEvent.keyDown(input, { key: 'Enter' });
        expect(screen.getByText('nature')).toBeTruthy();
    });
});
