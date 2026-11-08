import { Component, For, Show, createSignal } from 'solid-js';
import { Plus, FolderPlus, Trash2, Layers } from 'lucide-solid';
import { Button } from '../../ui';
import { type SearchGroup } from '../../../core/store/filter';
import { cn } from '../../../lib/utils';
import { useAdvancedSearch } from './useAdvancedSearch';
import { LogicOperatorToggle } from './LogicOperatorToggle';
import { CriterionRow } from './CriterionRow';
import { CriteriaBuilderRow } from './CriteriaBuilderRow';

/**
 * Properties for the QueryGroupCard component.
 */
interface QueryGroupCardProperties {
    /** The search group configuration containing items and logical operators. */
    group: SearchGroup;
    /** The active search context providing state and manipulation actions. */
    search: ReturnType<typeof useAdvancedSearch>;
    /** Current nesting depth, starting at 0 for the root container. */
    depth: number;
}

/**
 * Recursive card component representing a logical grouping of search rules or nested sub-groups.
 * Supports unlimited recursive depths with clean surface elevations and indented structural guides.
 *
 * @param properties - Properties for the component.
 * @returns The rendered group container.
 */
export const QueryGroupCard: Component<QueryGroupCardProperties> = properties => {
    const isRootGroup = () => properties.group.id === properties.search.rootGroup().id;
    const [isComposerActive, setIsComposerActive] = createSignal(false);

    const handleCreateNestedGroup = () => {
        properties.search.handleAddGroup(properties.group.id, 'and');
    };

    const handleRemoveThisGroup = () => {
        properties.search.handleRemoveGroup(properties.group.id);
    };

    return (
        <section
            class={cn(
                'query-group-card',
                isRootGroup() ? 'is-root-group' : 'is-nested-group',
                properties.depth % 2 === 1 ? 'depth-surface-odd' : 'depth-surface-even'
            )}
            role="group"
            aria-label={
                isRootGroup() ? 'Main Search Query' : `Nested Group (Level ${properties.depth})`
            }
        >
            <header class="query-group-header">
                <div class="query-group-title-group">
                    <span class="query-group-icon" aria-hidden="true">
                        <Layers size={14} />
                    </span>
                    <span class="query-group-heading">
                        {isRootGroup()
                            ? 'Match Conditions'
                            : `Subgroup (Level ${properties.depth})`}
                    </span>
                    <LogicOperatorToggle
                        logicalOperator={properties.group.logicalOperator}
                        onOperatorChange={selectedOperator =>
                            properties.search.handleSetGroupOperator(
                                properties.group.id,
                                selectedOperator
                            )
                        }
                        isCompact={properties.depth > 0}
                    />
                </div>

                <Show when={!isRootGroup()}>
                    <Button
                        variant="ghost-destructive"
                        size="icon-xs"
                        title="Remove subgroup"
                        aria-label="Remove subgroup"
                        onClick={handleRemoveThisGroup}
                    >
                        <Trash2 size={13} />
                    </Button>
                </Show>
            </header>

            <div class="query-group-items-container" role="list">
                <Show when={properties.group.items.length === 0 && !isComposerActive()}>
                    <div class="query-group-empty-state">
                        <p class="query-group-empty-text">No conditions in this group yet.</p>
                        <Button
                            variant="outline"
                            size="sm"
                            onClick={() => setIsComposerActive(true)}
                            leftIcon={<Plus size={14} />}
                        >
                            Add Condition
                        </Button>
                    </div>
                </Show>

                <For each={properties.group.items}>
                    {searchItem =>
                        'key' in searchItem ? (
                            <CriterionRow
                                criterion={searchItem}
                                search={properties.search}
                                onRemove={() =>
                                    properties.search.handleRemoveCriteria(searchItem.id)
                                }
                            />
                        ) : (
                            <QueryGroupCard
                                group={searchItem}
                                search={properties.search}
                                depth={properties.depth + 1}
                            />
                        )
                    }
                </For>
            </div>

            <Show when={isComposerActive()}>
                <div class="query-group-composer-wrapper">
                    <CriteriaBuilderRow
                        onAdd={newCriterion => {
                            properties.search.handleAddItem(properties.group.id, newCriterion);
                            setIsComposerActive(false);
                        }}
                        onCancel={() => setIsComposerActive(false)}
                    />
                </div>
            </Show>

            <footer class="query-group-action-footer">
                <Show when={!isComposerActive()}>
                    <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => setIsComposerActive(true)}
                        leftIcon={<Plus size={14} />}
                    >
                        Add Condition
                    </Button>
                </Show>

                <Button
                    variant="ghost"
                    size="sm"
                    onClick={handleCreateNestedGroup}
                    leftIcon={<FolderPlus size={14} />}
                >
                    Add Subgroup
                </Button>
            </footer>
        </section>
    );
};
