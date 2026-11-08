import { Component, createMemo, Show } from 'solid-js';
import { RotateCcw } from 'lucide-solid';
import { Button, Badge } from '../../ui';
import { useAdvancedSearch } from './useAdvancedSearch';
import { QueryGroupCard } from './QueryGroupCard';

/**
 * Properties for the QueryEditor component.
 */
export interface QueryEditorProperties {
    /** The active search context containing state signals and mutator actions. */
    search: ReturnType<typeof useAdvancedSearch>;
}

/**
 * Main query builder section that renders the root boolean group and overall query controls.
 *
 * @param componentProperties - Properties for the component.
 * @returns The rendered query editor layout.
 */
export const QueryEditor: Component<QueryEditorProperties> = componentProperties => {
    const totalCriteriaCount = createMemo(() => componentProperties.search.criteriaCount());

    return (
        <div class="query-editor-section" role="region" aria-label="Search query rules">
            <div class="query-editor-header-bar">
                <div class="query-editor-title-container">
                    <span class="query-editor-section-title">Search Rules</span>
                    <Badge variant="secondary" size="sm">
                        {totalCriteriaCount() === 1
                            ? '1 condition'
                            : `${totalCriteriaCount()} conditions`}
                    </Badge>
                </div>

                <Show when={totalCriteriaCount() > 0}>
                    <Button
                        variant="ghost"
                        size="xs"
                        onClick={componentProperties.search.handleReset}
                        leftIcon={<RotateCcw size={12} />}
                        title="Clear all conditions"
                    >
                        Reset rules
                    </Button>
                </Show>
            </div>

            <div class="query-editor-card-container">
                <QueryGroupCard
                    group={componentProperties.search.rootGroup()}
                    search={componentProperties.search}
                    depth={0}
                />
            </div>
        </div>
    );
};
