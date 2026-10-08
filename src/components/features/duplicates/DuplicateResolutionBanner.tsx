import { Component, Show } from 'solid-js';
import { Button } from '../../ui';
import { Undo2 } from 'lucide-solid';
import { DuplicateGroupStatus } from './types';

export interface DuplicateResolutionBannerProperties {
    /** Current status of the duplicate group */
    groupStatus: DuplicateGroupStatus;
    /** Whether an action is currently being executed */
    isProcessing: boolean;
    /** Callback triggered when the undo resolution or reopen action is clicked */
    onUndoAction: () => Promise<void> | void;
}

/**
 * Banner displayed inside the comparison panel when viewing a resolved or ignored duplicate group.
 * Provides contextual explanation and an action button to undo resolution or reopen the group.
 *
 * @param {DuplicateResolutionBannerProperties} properties - Component properties.
 * @returns {JSX.Element} The rendered banner component.
 */
export const DuplicateResolutionBanner: Component<
    DuplicateResolutionBannerProperties
> = properties => {
    return (
        <Show when={properties.groupStatus !== 'open'}>
            <Show
                when={properties.groupStatus === 'resolved'}
                fallback={
                    <div class="resolved-group-banner is-ignored" role="status">
                        <div class="resolved-group-banner-content">
                            <span class="resolved-group-banner-title">
                                This duplicate group is ignored.
                            </span>
                            <span class="resolved-group-banner-description">
                                Ignored groups are hidden by default from duplicate scans. You can
                                reopen it to review candidates.
                            </span>
                        </div>
                        <Button
                            variant="secondary"
                            size="sm"
                            onClick={properties.onUndoAction}
                            disabled={properties.isProcessing}
                        >
                            <Undo2 size={14} class="duplicate-comparison-button-icon" />
                            Reopen Group
                        </Button>
                    </div>
                }
            >
                <div class="resolved-group-banner" role="status">
                    <div class="resolved-group-banner-content">
                        <span class="resolved-group-banner-title">
                            This duplicate group has been resolved.
                        </span>
                        <span class="resolved-group-banner-description">
                            Discarded candidates were moved to trash. You can undo this resolution
                            at any time to restore the assets back to your library.
                        </span>
                    </div>
                    <Button
                        variant="secondary"
                        size="sm"
                        onClick={properties.onUndoAction}
                        disabled={properties.isProcessing}
                    >
                        <Undo2 size={14} class="duplicate-comparison-button-icon" />
                        Undo Resolution
                    </Button>
                </div>
            </Show>
        </Show>
    );
};
