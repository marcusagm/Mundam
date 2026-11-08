import { Component, createSignal, createEffect, Show } from 'solid-js';
import { Search, Save, FolderHeart, X } from 'lucide-solid';
import { Button, Input, Modal } from '../../ui';
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
    /** Whether the modal dialog is currently visible. */
    isOpen: boolean;
    /** Callback invoked when closing the modal. */
    onClose: () => void;
    /** Whether the modal is in Smart Folder authoring/editing mode. */
    isSmartFolderMode?: boolean;
    /** Optional identifier for the smart folder being edited. */
    initialIdentifier?: string;
    /** Initial name of the smart folder. */
    initialName?: string;
    /** Initial search query configuration. */
    initialQuery?: SearchGroup;
    /** Callback to persist the query as a smart folder. */
    onSave?: (
        folderName: string,
        searchQuery: SearchGroup,
        folderIdentifier?: string
    ) => Promise<ActionResult<void>> | ActionResult<void>;
}

/**
 * Modal dialog for creating and editing advanced multi-criteria search queries and smart folders.
 *
 * @param componentProperties - Properties for the component.
 * @returns The rendered AdvancedSearchModal.
 */
export const AdvancedSearchModal: Component<
    AdvancedSearchModalProperties
> = componentProperties => {
    const filtersStore = useFilters();

    const [smartFolderName, setSmartFolderName] = createSignal(
        componentProperties.initialName || ''
    );
    const [isSaveSmartFolderExpanded, setIsSaveSmartFolderExpanded] = createSignal(false);

    const advancedSearch = useAdvancedSearch({
        isOpen: () => componentProperties.isOpen,
        initialQuery: () => componentProperties.initialQuery
    });

    createEffect(() => {
        if (componentProperties.isOpen) {
            setSmartFolderName(componentProperties.initialName || '');
            setIsSaveSmartFolderExpanded(false);
        }
    });

    /** Executes the search query and closes the dialog. */
    const handleExecuteSearch = () => {
        if (advancedSearch.criteriaCount() === 0) {
            return;
        }
        const compiledQuery: SearchGroup = advancedSearch.buildQuery();
        filtersStore.setAdvancedSearch(compiledQuery);
        componentProperties.onClose();
    };

    /** Saves current query criteria as a smart folder. */
    const handleSaveSmartFolder = async () => {
        const trimmedName = smartFolderName().trim();
        if (!trimmedName || advancedSearch.criteriaCount() === 0) {
            return;
        }

        const compiledQuery: SearchGroup = advancedSearch.buildQuery();
        await componentProperties.onSave?.(
            trimmedName,
            compiledQuery,
            componentProperties.initialIdentifier
        );
        componentProperties.onClose();
    };

    const modalTitle = () => {
        if (componentProperties.isSmartFolderMode) {
            return componentProperties.initialIdentifier ? 'Edit Smart Folder' : 'New Smart Folder';
        }
        return 'Advanced Search';
    };

    return (
        <Modal
            isOpen={componentProperties.isOpen}
            onClose={componentProperties.onClose}
            title={modalTitle()}
            class="advanced-search-modal"
            size="xl"
            footer={
                <div class="advanced-search-modal-footer">
                    <Button variant="ghost" onClick={componentProperties.onClose}>
                        Cancel
                    </Button>

                    <div class="modal-footer-spacer" />

                    <Show
                        when={!componentProperties.isSmartFolderMode}
                        fallback={
                            <Button
                                variant="primary"
                                onClick={handleSaveSmartFolder}
                                disabled={
                                    advancedSearch.criteriaCount() === 0 ||
                                    !smartFolderName().trim()
                                }
                                leftIcon={<Save size={16} />}
                            >
                                {componentProperties.initialIdentifier
                                    ? 'Save Changes'
                                    : 'Create Smart Folder'}
                            </Button>
                        }
                    >
                        <div class="smart-folder-save-section">
                            <Show
                                when={isSaveSmartFolderExpanded()}
                                fallback={
                                    <Button
                                        variant="outline"
                                        size="sm"
                                        onClick={() => setIsSaveSmartFolderExpanded(true)}
                                        leftIcon={<FolderHeart size={14} />}
                                    >
                                        Save as Smart Folder
                                    </Button>
                                }
                            >
                                <div class="smart-folder-inline-composer">
                                    <Input
                                        placeholder="Smart folder name..."
                                        value={smartFolderName()}
                                        onInput={event =>
                                            setSmartFolderName(event.currentTarget.value)
                                        }
                                        size="sm"
                                        wrapperClass="smart-folder-inline-input"
                                        autofocus
                                    />
                                    <Button
                                        variant="secondary"
                                        size="sm"
                                        onClick={handleSaveSmartFolder}
                                        disabled={
                                            advancedSearch.criteriaCount() === 0 ||
                                            !smartFolderName().trim()
                                        }
                                        leftIcon={<Save size={13} />}
                                    >
                                        Save
                                    </Button>
                                    <Button
                                        variant="ghost"
                                        size="icon-xs"
                                        onClick={() => setIsSaveSmartFolderExpanded(false)}
                                        title="Cancel saving smart folder"
                                        aria-label="Cancel saving smart folder"
                                    >
                                        <X size={13} />
                                    </Button>
                                </div>
                            </Show>
                        </div>

                        <Button
                            variant="primary"
                            onClick={handleExecuteSearch}
                            disabled={advancedSearch.criteriaCount() === 0}
                            leftIcon={<Search size={16} />}
                        >
                            {advancedSearch.criteriaCount() > 0
                                ? `Search Library (${advancedSearch.criteriaCount()})`
                                : 'Search Library'}
                        </Button>
                    </Show>
                </div>
            }
        >
            <div class="advanced-search-modal-content">
                <Show when={componentProperties.isSmartFolderMode}>
                    <div class="smart-folder-metadata-card">
                        <div class="smart-folder-metadata-header">
                            <FolderHeart size={16} class="smart-folder-metadata-icon" />
                            <div>
                                <h4 class="smart-folder-metadata-title">Smart Folder Details</h4>
                                <p class="smart-folder-metadata-description">
                                    Assets matching the conditions below will be automatically
                                    indexed in this folder.
                                </p>
                            </div>
                        </div>

                        <Input
                            placeholder="e.g. Highlights 2026, Unprocessed RAWs, High Rating..."
                            value={smartFolderName()}
                            onInput={event => setSmartFolderName(event.currentTarget.value)}
                        />
                    </div>
                </Show>

                <QueryEditor search={advancedSearch} />
            </div>
        </Modal>
    );
};
