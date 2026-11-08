import { cleanup, fireEvent, render, screen, waitFor } from '@solidjs/testing-library';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { type JSX } from 'solid-js';
import { AdvancedSearchModal } from './AdvancedSearchModal';
import { type SearchGroup } from '../../../core/store/filter';
import { InputProvider } from '../../../core/input/context';

const mockFilterActions = vi.hoisted(() => ({
    setAdvancedSearch: vi.fn()
}));

vi.mock('@tauri-apps/api/core', () => ({
    invoke: vi.fn().mockResolvedValue(null)
}));

vi.mock('../../../core/hooks', () => ({
    useFilters: () => ({
        setAdvancedSearch: mockFilterActions.setAdvancedSearch
    }),
    useMetadata: () => ({
        locations: [
            { id: '1', parent_id: null, name: 'Photos', path: '/photos' },
            { id: '2', parent_id: '1', name: 'Trips', path: '/photos/trips' }
        ],
        tags: [
            { id: 'tag-1', name: 'Nature', color: '#00ff00' },
            { id: 'tag-2', name: 'Landscape', color: '#0000ff' }
        ],
        smartFolders: []
    })
}));

const renderWithInputProvider = (renderFunction: () => JSX.Element) =>
    render(renderFunction, { wrapper: InputProvider });

afterEach(cleanup);
beforeEach(() => {
    mockFilterActions.setAdvancedSearch.mockReset();
});

describe('AdvancedSearchModal component', () => {
    it('renders the advanced search dialog with search conditions and logic operator switch', () => {
        const handleCloseModal = vi.fn();
        renderWithInputProvider(() => (
            <AdvancedSearchModal isOpen={true} onClose={handleCloseModal} />
        ));

        expect(screen.getByText('Advanced Search')).toBeTruthy();
        expect(screen.getByText('Search Rules')).toBeTruthy();
        expect(screen.getByText('Match Conditions')).toBeTruthy();
        expect(screen.getByText('All conditions (AND)')).toBeTruthy();
        expect(screen.getByText('Any condition (OR)')).toBeTruthy();
    });

    it('allows toggling between AND and OR logical operators', () => {
        const handleCloseModal = vi.fn();
        renderWithInputProvider(() => (
            <AdvancedSearchModal isOpen={true} onClose={handleCloseModal} />
        ));

        const orButton = screen.getByText('Any condition (OR)');
        fireEvent.click(orButton);
        expect(orButton.getAttribute('aria-checked')).toBe('true');

        const andButton = screen.getByText('All conditions (AND)');
        fireEvent.click(andButton);
        expect(andButton.getAttribute('aria-checked')).toBe('true');
    });

    it('renders preloaded query criteria and executes search', () => {
        const handleCloseModal = vi.fn();
        const initialSearchQuery: SearchGroup = {
            id: 'root-group-test',
            logicalOperator: 'and',
            items: [
                {
                    id: 'criterion-test-1',
                    key: 'filename',
                    operator: 'contains',
                    value: 'mountains',
                    displayValue: 'mountains'
                }
            ]
        };

        renderWithInputProvider(() => (
            <AdvancedSearchModal
                isOpen={true}
                onClose={handleCloseModal}
                initialQuery={initialSearchQuery}
            />
        ));

        expect(screen.getByText('1 condition')).toBeTruthy();
        expect(screen.getByText('mountains')).toBeTruthy();

        const searchButton = screen.getByText('Search Library (1)');
        fireEvent.click(searchButton);

        expect(mockFilterActions.setAdvancedSearch).toHaveBeenCalledTimes(1);
        expect(handleCloseModal).toHaveBeenCalledTimes(1);
    });

    it('supports adding nested subgroups', () => {
        const handleCloseModal = vi.fn();
        renderWithInputProvider(() => (
            <AdvancedSearchModal isOpen={true} onClose={handleCloseModal} />
        ));

        const addSubgroupButton = screen.getByText('Add Subgroup');
        fireEvent.click(addSubgroupButton);

        expect(screen.getByText('Subgroup (Level 1)')).toBeTruthy();
    });

    it('renders in smart folder mode with dedicated metadata card', async () => {
        const handleCloseModal = vi.fn();
        const handleSaveSmartFolder = vi.fn().mockResolvedValue({ success: true });
        const initialSearchQuery: SearchGroup = {
            id: 'root-smart-folder-test',
            logicalOperator: 'and',
            items: [
                {
                    id: 'criterion-sf-1',
                    key: 'filename',
                    operator: 'contains',
                    value: 'holiday',
                    displayValue: 'holiday'
                }
            ]
        };

        renderWithInputProvider(() => (
            <AdvancedSearchModal
                isOpen={true}
                onClose={handleCloseModal}
                isSmartFolderMode={true}
                initialName="Vacation 2026"
                initialIdentifier="folder-identifier-1"
                initialQuery={initialSearchQuery}
                onSave={handleSaveSmartFolder}
            />
        ));

        expect(screen.getByText('Edit Smart Folder')).toBeTruthy();
        expect(screen.getByText('Smart Folder Details')).toBeTruthy();

        const saveChangesButton = screen.getByText('Save Changes');
        fireEvent.click(saveChangesButton);

        await waitFor(() => {
            expect(handleSaveSmartFolder).toHaveBeenCalledWith(
                'Vacation 2026',
                expect.any(Object),
                'folder-identifier-1'
            );
            expect(handleCloseModal).toHaveBeenCalledTimes(1);
        });
    });

    it('supports expanding the inline smart folder save bar in search mode', async () => {
        const handleCloseModal = vi.fn();
        const handleSaveSmartFolder = vi.fn().mockResolvedValue({ success: true });
        const initialSearchQuery: SearchGroup = {
            id: 'root-test-save-bar',
            logicalOperator: 'and',
            items: [
                {
                    id: 'criterion-bar-1',
                    key: 'filename',
                    operator: 'contains',
                    value: 'sunshine',
                    displayValue: 'sunshine'
                }
            ]
        };

        renderWithInputProvider(() => (
            <AdvancedSearchModal
                isOpen={true}
                onClose={handleCloseModal}
                initialQuery={initialSearchQuery}
                onSave={handleSaveSmartFolder}
            />
        ));

        const saveAsSmartFolderButton = screen.getByText('Save as Smart Folder');
        fireEvent.click(saveAsSmartFolderButton);

        const folderNameInput = screen.getByPlaceholderText('Smart folder name...');
        expect(folderNameInput).toBeTruthy();

        fireEvent.input(folderNameInput, { target: { value: 'My Sunny Folder' } });

        const confirmSaveButton = screen.getByText('Save');
        fireEvent.click(confirmSaveButton);

        await waitFor(() => {
            expect(handleSaveSmartFolder).toHaveBeenCalledWith(
                'My Sunny Folder',
                expect.any(Object),
                undefined
            );
            expect(handleCloseModal).toHaveBeenCalledTimes(1);
        });
    });
});
