import { Button, Input, Modal } from '../../ui';
import { Component, createSignal, createEffect } from 'solid-js';
import { Search, Save } from 'lucide-solid';
import { useFilters } from '../../../core/hooks';
import { SearchGroup } from '../../../core/store/filter';
import { ActionResult } from '../../../core/types/actions';
import { useAdvancedSearch } from './useAdvancedSearch';
import { QueryEditor } from './QueryEditor';
import './advanced-search-modal.css';

/**
 * Properties for the AdvancedSearchModal component.
 */
interface AdvancedSearchModalProperties {
    /** Whether the modal is currently open. */
    isOpen: boolean;
    /** Callback invoked when the modal requests closure. */
    onClose: () => void;
    /** Whether the modal is in 'Smart Folder' creation/edit mode. */
    isSmartFolderMode?: boolean;
    /** Optional identifier for the smart folder being edited. */
    initialIdentifier?: string;
    /** Initial name of the smart folder. */
    initialName?: string;
    /** Initial search query configuration. */
    initialQuery?: SearchGroup;
    /** Callback to save a smart folder. */
    onSave?: (
        name: string,
        query: SearchGroup,
        identifier?: string
    ) => Promise<ActionResult<void>> | ActionResult<void>;
}

/**
 * Modal dialog for building complex search queries and managing smart folders.
 *
 * @param componentProperties - Properties for the component.
 * @returns The rendered AdvancedSearchModal.
 */
export const AdvancedSearchModal: Component<
    AdvancedSearchModalProperties
> = componentProperties => {
    const filters = useFilters();

    const [smartFolderName, setSmartFolderName] = createSignal(
        componentProperties.initialName || ''
    );

    const search = useAdvancedSearch({
        isOpen: () => componentProperties.isOpen,
        initialQuery: () => componentProperties.initialQuery
    });

    createEffect(() => {
        if (componentProperties.isOpen) {
            setSmartFolderName(componentProperties.initialName || '');
        }
    });

    /**
     * Executes the advanced search query.
     */
    const handleSearch = () => {
        const searchGroup: SearchGroup = search.buildQuery();
        filters.setAdvancedSearch(searchGroup);
        componentProperties.onClose();
    };

    /**
     * Saves the current search criteria as a smart folder.
     */
    const handleSaveSmartFolder = async () => {
        if (!smartFolderName().trim()) {
            return;
        }

        const searchGroup: SearchGroup = search.buildQuery();
        await componentProperties.onSave?.(
            smartFolderName().trim(),
            searchGroup,
            componentProperties.initialIdentifier
        );
        componentProperties.onClose();
    };

    return (
        <Modal
            isOpen={componentProperties.isOpen}
            onClose={componentProperties.onClose}
            title={componentProperties.isSmartFolderMode ? 'Edit Smart Folder' : 'Advanced Search'}
            class="advanced-search-modal"
            size="xl"
            footer={
                <div class="modal-footer-content">
                    <Button variant="ghost" onClick={componentProperties.onClose}>
                        Cancel
                    </Button>
                    <div style={{ flex: 1 }} />
                    {componentProperties.isSmartFolderMode ? (
                        <Button
                            onClick={handleSaveSmartFolder}
                            disabled={search.criteriaCount() === 0 || !smartFolderName().trim()}
                            leftIcon={<Save />}
                        >
                            Save Smart Folder
                        </Button>
                    ) : (
                        <>
                            <div class="smart-folder-creator">
                                <Input
                                    placeholder="Smart Folder Name..."
                                    value={smartFolderName()}
                                    onInput={event => setSmartFolderName(event.currentTarget.value)}
                                    size="sm"
                                    wrapperClass="smart-folder-input-wrapper"
                                />
                                <Button
                                    onClick={handleSaveSmartFolder}
                                    disabled={
                                        search.criteriaCount() === 0 || !smartFolderName().trim()
                                    }
                                    variant="outline"
                                    leftIcon={<Save />}
                                >
                                    Save as Smart Folder
                                </Button>
                            </div>
                            <Button
                                onClick={handleSearch}
                                disabled={search.criteriaCount() === 0}
                                leftIcon={<Search />}
                            >
                                Search
                            </Button>
                        </>
                    )}
                </div>
            }
        >
            <div class="advanced-search-modal-content">
                {componentProperties.isSmartFolderMode && (
                    <div class="smart-folder-name-section">
                        <label class="section-title">Smart Folder Name</label>
                        <Input
                            placeholder="My awesome compilation..."
                            value={smartFolderName()}
                            onInput={event => setSmartFolderName(event.currentTarget.value)}
                        />
                    </div>
                )}

                <QueryEditor search={search} />
            </div>
        </Modal>
    );
};
