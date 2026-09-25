import { Component, For, Accessor, Setter } from 'solid-js';
import { Filter } from 'lucide-solid';
import { Button } from '../../ui';
import { DropdownMenu } from '../../ui/DropdownMenu';
import { DuplicateGroup } from './types';
import { DuplicateGroupItem } from './DuplicateGroupItem';
import '../../ui/SidebarPanel/sidebar-panel.css';
import './duplicate-group-list.css';

export interface DuplicateGroupListProperties {
    groups: DuplicateGroup[];
    selectedGroupId: string | null;
    onSelectGroup: (groupId: string) => void;
    showIgnored: Accessor<boolean>;
    setShowIgnored: Setter<boolean>;
    groupTypeFilter: Accessor<'all' | 'exact' | 'visual' | 'derived'>;
    setGroupTypeFilter: Setter<'all' | 'exact' | 'visual' | 'derived'>;
}

export const DuplicateGroupList: Component<DuplicateGroupListProperties> = props => {
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
                            { type: 'separator' },
                            {
                                type: 'checkbox',
                                label: 'All types',
                                checked: props.groupTypeFilter() === 'all',
                                onCheckedChange: checked => {
                                    if (checked) props.setGroupTypeFilter('all');
                                }
                            },
                            {
                                type: 'checkbox',
                                label: 'Exact match',
                                checked: props.groupTypeFilter() === 'exact',
                                onCheckedChange: checked => {
                                    if (checked) props.setGroupTypeFilter('exact');
                                }
                            },
                            {
                                type: 'checkbox',
                                label: 'Visual match',
                                checked: props.groupTypeFilter() === 'visual',
                                onCheckedChange: checked => {
                                    if (checked) props.setGroupTypeFilter('visual');
                                }
                            },
                            {
                                type: 'checkbox',
                                label: 'Derived / Edited',
                                checked: props.groupTypeFilter() === 'derived',
                                onCheckedChange: checked => {
                                    if (checked) props.setGroupTypeFilter('derived');
                                }
                            }
                        ]}
                    />
                </div>
            </header>
            <div class="group-list-container">
                <For each={props.groups}>
                    {group => (
                        <DuplicateGroupItem
                            group={group}
                            isSelected={props.selectedGroupId === group.id}
                            onSelect={() => props.onSelectGroup(group.id)}
                        />
                    )}
                </For>
            </div>
        </>
    );
};
