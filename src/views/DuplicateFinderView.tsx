import { Component, JSX, Show, createMemo, createSignal } from 'solid-js';
import { RefreshCw, Search, Bell, Settings } from 'lucide-solid';
import { ResizablePanelGroup, ResizablePanel, ResizableHandle } from '../components/ui';
import {
    DuplicateGroupList,
    DuplicateComparisonPanel,
    DuplicateRulesModal
} from '../components/features/duplicates';
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
        groupTypeFilter,
        setGroupTypeFilter,
        isScanning,
        scanProgress,
        newGroupsFoundCount,
        resetNewGroupsCount
    } = useDuplicateGroups();

    const [isRulesModalOpen, setIsRulesModalOpen] = createSignal(false);

    const selectedGroup = () => {
        const currentGroups = groups();
        if (!currentGroups) return undefined;
        return currentGroups.find(group => group.id === selectedGroupId());
    };

    /**
     * Derives the real progress percentage when `total > 0`.
     * Returns `undefined` when indeterminate (total is 0 or progress is null).
     */
    const scanProgressValue = createMemo(() => {
        const progress = scanProgress();
        if (!progress || progress.total === 0) return undefined;
        return (progress.processed / progress.total) * 100;
    });

    createShortcut({
        keys: 'ArrowDown',
        scope: 'viewport',
        system: true,
        action: () => {
            const currentVisible = visibleGroups();
            if (!currentVisible || currentVisible.length === 0) return;
            const currentIndex = currentVisible.findIndex(group => group.id === selectedGroupId());
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
            const currentIndex = currentVisible.findIndex(group => group.id === selectedGroupId());
            const prevIndex = currentIndex > 0 ? currentIndex - 1 : 0;
            if (currentVisible[prevIndex]) selectGroup(currentVisible[prevIndex].id);
        }
    });

    return (
        <div class="duplicate-finder">
            {props.header}

            <Show when={newGroupsFoundCount() > 0}>
                <div class="duplicate-finder-new-groups-banner" role="alert">
                    <Bell size={14} />
                    <span>
                        {newGroupsFoundCount()} new duplicate group
                        {newGroupsFoundCount() > 1 ? 's' : ''} found in background scan.
                    </span>
                    <button
                        class="duplicate-finder-banner-dismiss"
                        onClick={resetNewGroupsCount}
                        aria-label="Dismiss notification"
                    >
                        Dismiss
                    </button>
                </div>
            </Show>

            <ResizablePanelGroup direction="horizontal" class="duplicate-finder-body">
                <ResizablePanel id="list-panel" defaultSize={30} minSize={20} maxSize={50}>
                    <div class="duplicate-finder-sidebar">
                        <div
                            class="duplicate-finder-toolbar"
                            role="toolbar"
                            aria-label="Duplicate actions"
                        >
                            <div class="duplicate-finder-toolbar-actions">
                                <Show
                                    when={isScanning()}
                                    fallback={
                                        <Button
                                            variant="primary"
                                            class="duplicate-finder-toolbar-button"
                                            onClick={startScan}
                                        >
                                            <Search
                                                size={16}
                                                class="duplicate-finder-toolbar-icon"
                                            />
                                            Scan Now
                                        </Button>
                                    }
                                >
                                    <Button
                                        variant="secondary"
                                        class="duplicate-finder-toolbar-button"
                                        onClick={cancelScan}
                                    >
                                        <RefreshCw
                                            size={16}
                                            class="duplicate-finder-toolbar-icon duplicate-finder-spinner"
                                        />
                                        Cancel Scan
                                    </Button>
                                </Show>

                                <Button
                                    variant="secondary"
                                    onClick={() => setIsRulesModalOpen(true)}
                                    title="Configure Rules"
                                    size="icon"
                                >
                                    <Settings size={16} />
                                </Button>
                            </div>

                            <Show when={isScanning()}>
                                <div class="duplicate-finder-scan-progress-row">
                                    <Show
                                        when={scanProgress()}
                                        fallback={<span>Preparing scan…</span>}
                                    >
                                        <span>
                                            {scanProgress()?.processed || 0}
                                            <Show when={scanProgressValue() !== undefined}>
                                                {' / '}
                                                {scanProgress()?.total}
                                            </Show>{' '}
                                            files hashed
                                        </span>
                                        <span>Groups: {scanProgress()?.groupsCreated || 0}</span>
                                    </Show>
                                </div>
                                <ProgressBar
                                    value={scanProgressValue() ?? 0}
                                    isIndeterminate={scanProgressValue() === undefined}
                                />
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
                                groupTypeFilter={groupTypeFilter}
                                setGroupTypeFilter={setGroupTypeFilter}
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

            <DuplicateRulesModal
                isOpen={isRulesModalOpen()}
                onClose={() => setIsRulesModalOpen(false)}
            />
        </div>
    );
};
