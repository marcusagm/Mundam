import { Component, JSX, Show } from 'solid-js';
import { RefreshCw, Search } from 'lucide-solid';
import { ResizablePanelGroup, ResizablePanel, ResizableHandle } from '../components/ui';
import { DuplicateGroupList, DuplicateComparisonPanel } from '../components/features/duplicates';
import { Button } from '../components/ui/Button';
import { ProgressBar } from '../components/ui/ProgressBar';
import { Loader } from '../components/ui/Loader';
import { useDuplicateGroups } from '../components/features/duplicates/hooks/useDuplicateGroups';
import { createShortcut } from '../core/input';

import './duplicate-finder-view.css';

export interface DuplicateFinderViewProperties {
    /** The title bar to render at the top */
    header: JSX.Element;
}

/**
 * The Duplicate Finder view interface.
 * Shows a list of duplicate groups on the left and group details/actions on the right.
 *
 * @param {DuplicateFinderViewProperties} props - Component properties.
 * @returns {JSX.Element} The rendered duplicate finder view.
 */
export const DuplicateFinderView: Component<DuplicateFinderViewProperties> = props => {
    const {
        groups,
        visibleGroups,
        selectedGroupId,
        selectGroup,
        resolveGroup,
        startScan,
        cancelScan,
        showIgnored,
        setShowIgnored,
        isScanning,
        scanProgress
    } = useDuplicateGroups();

    const selectedGroup = () => {
        const currentGroups = groups();
        if (!currentGroups) return undefined;
        return currentGroups.find(group => group.id === selectedGroupId());
    };

    createShortcut({
        keys: 'ArrowDown',
        scope: 'viewport',
        system: true,
        action: () => {
            const currentVisible = visibleGroups();
            if (!currentVisible || currentVisible.length === 0) return;
            const currentIndex = currentVisible.findIndex(g => g.id === selectedGroupId());
            const nextIndex =
                currentIndex < currentVisible.length - 1 ? currentIndex + 1 : currentIndex;
            if (currentVisible[nextIndex]) selectGroup(currentVisible[nextIndex].id);
        }
    });

    createShortcut({
        keys: 'ArrowUp',
        scope: 'viewport',
        system: true,
        action: () => {
            const currentVisible = visibleGroups();
            if (!currentVisible || currentVisible.length === 0) return;
            const currentIndex = currentVisible.findIndex(g => g.id === selectedGroupId());
            const prevIndex = currentIndex > 0 ? currentIndex - 1 : 0;
            if (currentVisible[prevIndex]) selectGroup(currentVisible[prevIndex].id);
        }
    });

    return (
        <div class="duplicate-finder">
            {props.header}
            <ResizablePanelGroup direction="horizontal" class="duplicate-finder-body">
                <ResizablePanel id="list-panel" defaultSize={30} minSize={20} maxSize={50}>
                    <div class="duplicate-finder-sidebar">
                        <div class="duplicate-finder-toolbar">
                            <Show
                                when={isScanning()}
                                fallback={
                                    <Button
                                        variant="primary"
                                        class="duplicate-finder-toolbar-button"
                                        onClick={startScan}
                                    >
                                        <Search size={16} class="mr-2" />
                                        Scan Now
                                    </Button>
                                }
                            >
                                <Button
                                    variant="secondary"
                                    class="duplicate-finder-toolbar-button"
                                    onClick={cancelScan}
                                >
                                    <RefreshCw size={16} class="mr-2 animate-spin" />
                                    Cancel Scan
                                </Button>
                            </Show>

                            <Show when={isScanning()}>
                                <div class="duplicate-finder-scan-progress-row">
                                    <Show
                                        when={scanProgress()}
                                        fallback={<span>Scanning library...</span>}
                                    >
                                        <span>Processed: {scanProgress()?.processed || 0}</span>
                                        <span>Groups: {scanProgress()?.groupsCreated || 0}</span>
                                    </Show>
                                </div>
                                <ProgressBar value={0} isIndeterminate={true} />
                            </Show>
                        </div>
                        <Show
                            when={!groups.loading}
                            fallback={
                                <div class="duplicate-finder-loading-state">
                                    <Loader size="md" />
                                    <span>Loading duplicates...</span>
                                </div>
                            }
                        >
                            <DuplicateGroupList
                                groups={visibleGroups()}
                                selectedGroupId={selectedGroupId()}
                                onSelectGroup={selectGroup}
                                showIgnored={showIgnored}
                                setShowIgnored={setShowIgnored}
                            />
                        </Show>
                    </div>
                </ResizablePanel>

                <ResizableHandle />

                <ResizablePanel id="details-panel" defaultSize={70}>
                    <div class="duplicate-finder-content">
                        <Show
                            when={
                                selectedGroup() && selectedGroup()!.candidatesLoaded
                                    ? selectedGroup()
                                    : undefined
                            }
                            fallback={
                                <div class="duplicate-finder-empty">
                                    <h2>
                                        {selectedGroupId()
                                            ? selectedGroup()?.candidatesLoaded
                                                ? 'No valid candidates found for this group.'
                                                : 'Loading candidates...'
                                            : 'Select a group'}
                                    </h2>
                                </div>
                            }
                        >
                            <Show
                                when={selectedGroup()!.candidates.length > 0}
                                fallback={
                                    <div class="duplicate-finder-empty">
                                        <p style={{ color: 'var(--text-tertiary)' }}>
                                            The assets in this group are missing or have been
                                            deleted.
                                        </p>
                                    </div>
                                }
                            >
                                <DuplicateComparisonPanel
                                    group={selectedGroup()!}
                                    onResolve={resolveGroup}
                                />
                            </Show>
                        </Show>
                    </div>
                </ResizablePanel>
            </ResizablePanelGroup>
        </div>
    );
};
