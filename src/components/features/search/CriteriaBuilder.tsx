import { Component } from 'solid-js';
import { type SearchCriterion } from '../../../core/store/filter';
import { CriteriaBuilderRow } from './CriteriaBuilderRow';

/**
 * Properties for backwards compatibility with CriteriaBuilder.
 */
export interface CriteriaBuilderProps {
    /** Called with a validated criterion when the user confirms the form. */
    onAdd: (criterion: SearchCriterion) => void;
}

/**
 * Backwards compatibility wrapper forwarding to CriteriaBuilderRow.
 *
 * @param properties - Properties for the component.
 * @returns The rendered criteria builder row.
 */
export const CriteriaBuilder: Component<CriteriaBuilderProps> = properties => {
    return <CriteriaBuilderRow onAdd={properties.onAdd} />;
};
