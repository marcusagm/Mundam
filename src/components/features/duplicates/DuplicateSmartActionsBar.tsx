import { Component, Show } from 'solid-js';
import { Button } from '../../ui';
import { DuplicateCandidate } from './types';

export interface DuplicateSmartActionsBarProperties {
    /** The candidate with the largest file size, if any */
    largestCandidate: DuplicateCandidate | null;
    /** The candidate created earliest, if any */
    oldestCandidate: DuplicateCandidate | null;
    /** The candidate marked as favorite, if any */
    favoriteCandidate: DuplicateCandidate | null;
    /** Whether an action is currently being processed */
    isProcessing: boolean;
    /** Callback triggered when a quick action candidate is chosen */
    onSelectCandidate: (candidate: DuplicateCandidate) => void;
}

/**
 * Bar containing quick resolution shortcuts (Keep Largest, Keep Oldest, Keep Favorite).
 *
 * @param {DuplicateSmartActionsBarProperties} properties - Component properties.
 * @returns {JSX.Element} The rendered quick actions bar.
 */
export const DuplicateSmartActionsBar: Component<
    DuplicateSmartActionsBarProperties
> = properties => {
    return (
        <div class="smart-actions-bar">
            <span class="smart-actions-label">Quick Actions</span>
            <div class="smart-actions-buttons">
                <Button
                    variant="secondary"
                    size="sm"
                    disabled={properties.isProcessing || !properties.largestCandidate}
                    onClick={() => {
                        if (properties.largestCandidate) {
                            properties.onSelectCandidate(properties.largestCandidate);
                        }
                    }}
                >
                    Keep Largest
                </Button>
                <Button
                    variant="secondary"
                    size="sm"
                    disabled={properties.isProcessing || !properties.oldestCandidate}
                    onClick={() => {
                        if (properties.oldestCandidate) {
                            properties.onSelectCandidate(properties.oldestCandidate);
                        }
                    }}
                >
                    Keep Oldest
                </Button>
                <Show when={properties.favoriteCandidate}>
                    <Button
                        variant="secondary"
                        size="sm"
                        disabled={properties.isProcessing}
                        onClick={() => {
                            if (properties.favoriteCandidate) {
                                properties.onSelectCandidate(properties.favoriteCandidate);
                            }
                        }}
                    >
                        Keep Favorite
                    </Button>
                </Show>
            </div>
        </div>
    );
};
