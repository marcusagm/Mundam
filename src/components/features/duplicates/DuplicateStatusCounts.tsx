import { createMemo, type Component } from 'solid-js';
import type { DuplicateGroup } from './types';

/** Summarizes all groups, the current list and the active comparison selection. */
export const DuplicateStatusCounts: Component<{
    groups?: DuplicateGroup[];
    shown: number;
    selected: number;
}> = props => {
    const counts = createMemo(() => {
        const groups = props.groups ?? [];
        return {
            total: groups.length,
            resolved: groups.filter(group => group.status === 'resolved').length,
            ignored: groups.filter(group => group.status === 'ignored').length
        };
    });
    return (
        <div class="statusbar-section" aria-label="Duplicate groups">
            <span title="All duplicate groups">Groups: {counts().total}</span>
            <span class="statusbar-divider" />
            <span title="Groups matching the current filters">Shown: {props.shown}</span>
            <span title="Resolved groups across all filters">Resolved: {counts().resolved}</span>
            <span title="Ignored groups across all filters">Ignored: {counts().ignored}</span>
            <span class="statusbar-divider" />
            <span class="statusbar-selected" title="Selected assets in this group">
                Assets selected: {props.selected}
            </span>
        </div>
    );
};
