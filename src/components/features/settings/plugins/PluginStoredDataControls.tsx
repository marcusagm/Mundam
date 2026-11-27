import { createResource, createSignal, For, Show, type Component } from 'solid-js';
import type {
    PluginStoredDataClient,
    StoredPluginDataCounts,
    StoredPluginOwner
} from '../../../../lib/plugins';
import { Button } from '../../../ui/Button';

interface PendingReview {
    owner: StoredPluginOwner;
    counts: StoredPluginDataCounts;
    revision: string;
}

function describeCounts(counts: StoredPluginDataCounts): string {
    return `${counts.annotations} saved · ${counts.recoveryAnnotations} in folder recovery · ${counts.cachedResults} cached`;
}

/** Confirms permanent removal of data for one reviewed identity. */
export const PluginStoredDataControls: Component<{
    client: PluginStoredDataClient;
    exportInProgress: boolean;
}> = properties => {
    const [expanded, setExpanded] = createSignal(false);
    const [inventory, { refetch }] = createResource(expanded, () =>
        properties.client.list().catch(() => null)
    );
    const [review, setReview] = createSignal<PendingReview>();
    const [busy, setBusy] = createSignal(false);
    const [status, setStatus] = createSignal('');
    const [error, setError] = createSignal('');
    const unavailable = () => properties.exportInProgress || busy();

    const run = async (action: () => Promise<void>, errorMessage: string) => {
        if (unavailable()) return;
        setBusy(true);
        setStatus('');
        setError('');
        try {
            await action();
        } catch {
            setError(errorMessage);
        } finally {
            setBusy(false);
        }
    };

    const reviewRemoval = (owner: StoredPluginOwner) =>
        run(async () => {
            setReview(undefined);
            const snapshot = await properties.client.review(owner);
            setReview({ owner, ...snapshot });
        }, 'Could not review stored plugin data.');

    const confirmRemoval = (pending: PendingReview) =>
        run(async () => {
            setReview(undefined);
            if (await properties.client.purge(pending.owner, pending.revision)) {
                setStatus('Stored plugin data removed from this catalog.');
            } else {
                setError('Stored data changed. Review the current data before removing it.');
            }
            await refetch();
        }, 'Could not remove stored plugin data. Review it and try again.');

    const toggle = () => {
        setReview(undefined);
        setStatus('');
        setError('');
        setExpanded(!expanded());
    };

    return (
        <>
            <div class="plugin-settings-actions">
                <Button
                    size="sm"
                    variant="outline"
                    disabled={unavailable()}
                    aria-expanded={expanded()}
                    onClick={toggle}
                >
                    {expanded() ? 'Hide stored data' : 'Manage stored data'}
                </Button>
            </div>
            <Show when={expanded()}>
                <Show when={inventory.loading}>
                    <p role="status">Loading stored plugin data…</p>
                </Show>
                <Show when={inventory() === null && !inventory.loading}>
                    <p role="alert">Could not load stored plugin data.</p>
                    <Button size="sm" variant="outline" onClick={() => void refetch()}>
                        Retry
                    </Button>
                </Show>
                <Show when={inventory()?.length === 0 && !inventory.loading}>
                    <p>No stored plugin data in this catalog.</p>
                </Show>
                <ul class="plugin-settings-list">
                    <For each={inventory()}>
                        {entry => (
                            <li class="plugin-settings-entry">
                                <strong>{entry.owner.pluginId}</strong>
                                <code>{entry.owner.publisherFingerprint}</code>
                                <p>{describeCounts(entry.counts)}</p>
                                <div class="plugin-settings-actions">
                                    <Button
                                        size="sm"
                                        variant="ghost-destructive"
                                        disabled={unavailable() || inventory.loading}
                                        onClick={() => void reviewRemoval(entry.owner)}
                                    >
                                        Review data removal
                                    </Button>
                                </div>
                            </li>
                        )}
                    </For>
                </ul>
                <Show when={review()} keyed>
                    {pending => (
                        <div class="plugin-settings-entry">
                            <p>
                                Permanently remove stored data for{' '}
                                <strong>{pending.owner.pluginId}</strong> from this catalog?
                            </p>
                            <code>{pending.owner.publisherFingerprint}</code>
                            <p>{describeCounts(pending.counts)}</p>
                            <p>
                                Folder Undo restores the files and any remaining annotations.
                                Restore a folder before exporting annotations held by its recovery
                                point.
                            </p>
                            <div class="plugin-settings-actions">
                                <Button
                                    size="sm"
                                    variant="destructive"
                                    disabled={unavailable()}
                                    onClick={() => void confirmRemoval(pending)}
                                >
                                    Permanently remove data
                                </Button>
                                <Button
                                    size="sm"
                                    variant="outline"
                                    onClick={() => setReview(undefined)}
                                >
                                    Cancel data removal
                                </Button>
                            </div>
                        </div>
                    )}
                </Show>
                <Show when={error()}>
                    <p role="alert">{error()}</p>
                </Show>
                <Show when={status()}>
                    <p role="status">{status()}</p>
                </Show>
            </Show>
        </>
    );
};
