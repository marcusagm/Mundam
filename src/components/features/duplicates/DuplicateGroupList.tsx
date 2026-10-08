import { Component, Accessor, Setter, createMemo } from 'solid-js';
import { Filter } from 'lucide-solid';
import { Button, VirtualList } from '../../ui';
import { DropdownMenu } from '../../ui/DropdownMenu';
import { DuplicateGroup } from './types';
import { DuplicateGroupItem } from './DuplicateGroupItem';
import '../../ui/SidebarPanel/sidebar-panel.css';
import './duplicate-group-list.css';

/**
 * Configuration properties for the DuplicateGroupList component.
 */
export interface DuplicateGroupListProperties {
    /** Complete array of duplicate groups after active filters are applied */
    groups: DuplicateGroup[];
    /** The ID of the currently selected group, or null if none is selected */
    selectedGroupId: string | null;
    /** Callback invoked when the user selects a group */
    onSelectGroup: (groupId: string) => void;
    /** Reactive accessor indicating whether ignored groups are included */
    showIgnored: Accessor<boolean>;
    /** Setter for updating the showIgnored flag */
    setShowIgnored: Setter<boolean>;
    /** Reactive accessor indicating whether resolved groups are included */
    showResolved: Accessor<boolean>;
    /** Setter for updating the showResolved flag */
    setShowResolved: Setter<boolean>;
    /** Reactive accessor for the active group type filter */
    groupTypeFilter: Accessor<'all' | 'exact' | 'visual' | 'derived'>;
    /** Setter for updating the active group type filter */
    setGroupTypeFilter: Setter<'all' | 'exact' | 'visual' | 'derived'>;
    /** Optional overscan count for virtualization buffer */
    overscanCount?: number;
    /** Optional callback invoked when visible groups change in the viewport */
    onVisibleGroupsChange?: (visibleGroupList: DuplicateGroup[]) => void;
    /** Callback invoked to preload candidate files and thumbnail decks for visible groups */
    preloadGroupCandidates?: (groupIdList: string[]) => Promise<void> | void;
}

/**
 * Sidebar component displaying the virtualized list of duplicate groups.
 *
 * Uses the generic `VirtualList` component to render only visible groups in the viewport
 * with configurable buffer overscan (~5 items). Automatically triggers preloading of
 * candidate asset metadata and thumbnail decks for groups as they scroll into view.
 *
 * @param {DuplicateGroupListProperties} props - Component configuration properties.
 * @returns {JSX.Element} The rendered duplicate group list sidebar panel.
 */
export const DuplicateGroupList: Component<DuplicateGroupListProperties> = props => {
    /**
     * Handles changes in visible items reported by the virtual list.
     * Identifies groups needing candidate asset metadata and triggers batch preloading.
     */
    const handleVisibleItemsChange = (visibleGroupList: DuplicateGroup[]): void => {
        props.onVisibleGroupsChange?.(visibleGroupList);

        const candidateGroupsToPreload = visibleGroupList
            .filter(groupItem => !groupItem.candidatesLoaded && groupItem.candidateCount > 0)
            .map(groupItem => groupItem.id);

        if (candidateGroupsToPreload.length > 0 && props.preloadGroupCandidates) {
            props.preloadGroupCandidates(candidateGroupsToPreload);
        }
    };

    /**
     * Resolves the numeric index of the currently selected group
     * to keep it scrolled into view during keyboard navigation.
     */
    const selectedGroupIndex = createMemo(() => {
        const targetGroupId = props.selectedGroupId;
        if (!targetGroupId) {
            return -1;
        }
        return props.groups.findIndex(groupItem => groupItem.id === targetGroupId);
    });

    return (
        <>
            <header class="ui-sidebar-panel-header">
                <h3 class="ui-sidebar-panel-title">Duplicate Groups</h3>
                <div class="ui-sidebar-panel-actions" role="group">
                    <DropdownMenu
                        align="end"
                        trigger={
                            <Button variant="ghost" size="icon-xs" title="Filter options">
                                <Filter size={14} />
                            </Button>
                        }
                        items={[
                            {
                                type: 'checkbox',
                                label: 'Show ignored groups',
                                checked: props.showIgnored(),
                                onCheckedChange: props.setShowIgnored
                            },
                            {
                                type: 'checkbox',
                                label: 'Show resolved groups',
                                checked: props.showResolved(),
                                onCheckedChange: props.setShowResolved
                            },
                            { type: 'separator' },
                            {
                                type: 'checkbox',
                                label: 'All types',
                                checked: props.groupTypeFilter() === 'all',
                                onCheckedChange: checked => {
                                    if (checked) {
                                        props.setGroupTypeFilter('all');
                                    }
                                }
                            },
                            {
                                type: 'checkbox',
                                label: 'Exact match',
                                checked: props.groupTypeFilter() === 'exact',
                                onCheckedChange: checked => {
                                    if (checked) {
                                        props.setGroupTypeFilter('exact');
                                    }
                                }
                            },
                            {
                                type: 'checkbox',
                                label: 'Visual match',
                                checked: props.groupTypeFilter() === 'visual',
                                onCheckedChange: checked => {
                                    if (checked) {
                                        props.setGroupTypeFilter('visual');
                                    }
                                }
                            },
                            {
                                type: 'checkbox',
                                label: 'Derived / Edited',
                                checked: props.groupTypeFilter() === 'derived',
                                onCheckedChange: checked => {
                                    if (checked) {
                                        props.setGroupTypeFilter('derived');
                                    }
                                }
                            }
                        ]}
                    />
                </div>
            </header>
            <VirtualList
                items={props.groups}
                itemHeight={150}
                gap={8}
                overscanCount={props.overscanCount ?? 5}
                autoScrollToIndex={selectedGroupIndex()}
                class="group-list-container"
                ariaLabel="Duplicate Groups List"
                keyField="id"
                onVisibleItemsChange={handleVisibleItemsChange}
                fallback={
                    <div class="group-list-empty-state">
                        <span>No duplicate groups found</span>
                    </div>
                }
            >
                {groupItem => (
                    <DuplicateGroupItem
                        group={groupItem}
                        isSelected={props.selectedGroupId === groupItem.id}
                        onSelect={() => props.onSelectGroup(groupItem.id)}
                    />
                )}
            </VirtualList>
        </>
    );
};
