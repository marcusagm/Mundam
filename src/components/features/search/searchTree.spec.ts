import { describe, expect, it } from 'vitest';
import { type SearchCriterion, type SearchGroup } from '../../../core/store/filter';
import {
    addItemToGroup,
    countCriteria,
    findCriterion,
    pruneEmptyGroups,
    removeCriterion,
    removeGroup,
    setGroupOperator,
    updateCriterion
} from './searchTree';

const criterion = (id: string): SearchCriterion => ({
    id,
    key: 'filename',
    operator: 'contains',
    value: id
});

const buildTree = (): SearchGroup => ({
    id: 'root',
    logicalOperator: 'and',
    items: [
        criterion('a'),
        { id: 'child', logicalOperator: 'or', items: [criterion('b'), criterion('c')] }
    ]
});

describe('searchTree', () => {
    it('adds items to nested groups without touching the others', () => {
        const next = addItemToGroup(buildTree(), 'child', criterion('d'));

        expect(countCriteria(next)).toBe(4);
        expect(next.items).toHaveLength(2);
        expect(findCriterion(next, 'd')).toBeDefined();
    });

    it('changes the operator of a single group', () => {
        const next = setGroupOperator(buildTree(), 'child', 'and');

        expect(next.logicalOperator).toBe('and');
        expect((next.items[1] as SearchGroup).logicalOperator).toBe('and');
        expect(setGroupOperator(buildTree(), 'root', 'or').logicalOperator).toBe('or');
    });

    it('removes nested groups but never the root', () => {
        expect(removeGroup(buildTree(), 'child').items).toHaveLength(1);
        expect(removeGroup(buildTree(), 'root').id).toBe('root');
        expect(removeGroup(buildTree(), 'root').items).toHaveLength(2);
    });

    it('removes and updates criteria at any depth', () => {
        expect(countCriteria(removeCriterion(buildTree(), 'b'))).toBe(2);

        const updated = updateCriterion(buildTree(), 'c', item => ({ ...item, value: 'x' }));
        expect(findCriterion(updated, 'c')?.value).toBe('x');
    });

    it('prunes groups left empty', () => {
        const emptied = removeCriterion(removeCriterion(buildTree(), 'b'), 'c');

        expect(emptied.items).toHaveLength(2);
        expect(pruneEmptyGroups(emptied).items).toHaveLength(1);
    });
});
