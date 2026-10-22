import {
    type LogicalOperator,
    type SearchCriterion,
    type SearchGroup
} from '../../../core/store/filter';

export type SearchItem = SearchGroup['items'][number];

export const isCriterion = (item: SearchItem): item is SearchCriterion => 'key' in item;

/** Creates an empty group with a collision-free id. */
export const createEmptyGroup = (logicalOperator: LogicalOperator = 'and'): SearchGroup => ({
    id: `group-${crypto.randomUUID()}`,
    logicalOperator,
    items: []
});

const mapGroup = (
    group: SearchGroup,
    groupId: string,
    update: (target: SearchGroup) => SearchGroup
): SearchGroup => {
    if (group.id === groupId) return update(group);
    return {
        ...group,
        items: group.items.map(item => (isCriterion(item) ? item : mapGroup(item, groupId, update)))
    };
};

/** Appends `item` to the group identified by `groupId`. */
export const addItemToGroup = (root: SearchGroup, groupId: string, item: SearchItem): SearchGroup =>
    mapGroup(root, groupId, group => ({ ...group, items: [...group.items, item] }));

/** Changes how the items of the group identified by `groupId` are combined. */
export const setGroupOperator = (
    root: SearchGroup,
    groupId: string,
    logicalOperator: LogicalOperator
): SearchGroup => mapGroup(root, groupId, group => ({ ...group, logicalOperator }));

/** Removes a nested group. The root group can never be removed. */
export const removeGroup = (root: SearchGroup, groupId: string): SearchGroup => {
    const prune = (group: SearchGroup): SearchGroup => ({
        ...group,
        items: group.items
            .filter(item => isCriterion(item) || item.id !== groupId)
            .map(item => (isCriterion(item) ? item : prune(item)))
    });
    return prune(root);
};

/** Removes a criterion anywhere in the tree. Groups are kept, even when left empty. */
export const removeCriterion = (root: SearchGroup, criterionId: string): SearchGroup => ({
    ...root,
    items: root.items
        .filter(item => !isCriterion(item) || item.id !== criterionId)
        .map(item => (isCriterion(item) ? item : removeCriterion(item, criterionId)))
});

/** Replaces a criterion anywhere in the tree. */
export const updateCriterion = (
    root: SearchGroup,
    criterionId: string,
    update: (criterion: SearchCriterion) => SearchCriterion
): SearchGroup => ({
    ...root,
    items: root.items.map(item => {
        if (!isCriterion(item)) return updateCriterion(item, criterionId, update);
        return item.id === criterionId ? update(item) : item;
    })
});

/** Finds a criterion anywhere in the tree. */
export const findCriterion = (
    group: SearchGroup,
    criterionId: string
): SearchCriterion | undefined => {
    for (const item of group.items) {
        if (isCriterion(item)) {
            if (item.id === criterionId) return item;
        } else {
            const match = findCriterion(item, criterionId);
            if (match) return match;
        }
    }
    return undefined;
};

/** Counts the criteria in the whole tree. */
export const countCriteria = (group: SearchGroup): number =>
    group.items.reduce((total, item) => total + (isCriterion(item) ? 1 : countCriteria(item)), 0);

/** Drops empty groups so they do not produce meaningless clauses when searching or saving. */
export const pruneEmptyGroups = (group: SearchGroup): SearchGroup => ({
    ...group,
    items: group.items
        .map(item => (isCriterion(item) ? item : pruneEmptyGroups(item)))
        .filter(item => isCriterion(item) || item.items.length > 0)
});
