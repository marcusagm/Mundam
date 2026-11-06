import { For, Show, createSignal, onMount, type Component } from 'solid-js';
import { Button, Input } from '../../ui';
import { invokeCommand } from '../../../lib/api';
import { executeLibraryOperation } from '../../../core/store/library/folderActions';

/** Persisted deletion projection; archive contents remain owned by the backend. */
interface RecoveryEntry {
    entryId: string;
    restorable: boolean;
    folderId: string;
    originalPath: string;
    createdAt: string;
    byteCount: number;
}

const GIBIBYTE = 1024 ** 3;

/** Restores persisted Undo entries and controls storage growth without automatic deletion.
 * @returns Recovery controls for the trash settings section.
 * @example <FolderRecoverySettings />
 */
export const FolderRecoverySettings: Component = () => {
    const [entries, setEntries] = createSignal<RecoveryEntry[]>([]);
    const [offset, setOffset] = createSignal(0);
    const [hasMore, setHasMore] = createSignal(false);
    const [isBusy, setIsBusy] = createSignal(false);
    const [error, setError] = createSignal('');
    const [budget, setBudget] = createSignal('50');

    onMount(async () => {
        try {
            const value = await invokeCommand<string | null>('get_setting', {
                key: 'folder_recovery_max_bytes'
            });
            if (value && Number(value) > 0) setBudget(String(Number(value) / GIBIBYTE));
        } catch {
            setError('Could not load the recovery storage budget.');
        }
    });

    const load = async (pageOffset = 0): Promise<void> => {
        if (isBusy()) return;
        setIsBusy(true);
        setError('');
        try {
            const page = await invokeCommand<RecoveryEntry[]>('folder_recovery_history', {
                offset: pageOffset
            });
            setEntries(pageOffset ? previous => [...previous, ...page] : page);
            setOffset(pageOffset + page.length);
            setHasMore(page.length === 50);
        } catch {
            setError('Could not load folder recovery history.');
        } finally {
            setIsBusy(false);
        }
    };

    const restore = async (entry: RecoveryEntry): Promise<void> => {
        if (isBusy()) return;
        setIsBusy(true);
        setError('');
        try {
            const result = await executeLibraryOperation({
                kind: 'restoreFolder',
                folderId: entry.folderId
            });
            setEntries(previous =>
                previous.filter(candidate => candidate.folderId !== entry.folderId)
            );
            setOffset(previous => Math.max(0, previous - 1));
            if (result.warning) setError(result.warning);
        } catch (failure) {
            setError(
                typeof failure === 'object' && failure && 'message' in failure
                    ? String(failure.message)
                    : 'Restore failed. The archive is retained for retry.'
            );
        } finally {
            setIsBusy(false);
        }
    };

    const saveBudget = async (): Promise<void> => {
        const bytes = Math.floor(Number(budget()) * GIBIBYTE);
        if (!Number.isSafeInteger(bytes) || bytes <= 0) {
            setError('Enter a positive storage budget.');
            return;
        }
        setIsBusy(true);
        setError('');
        try {
            await invokeCommand('set_setting', {
                key: 'folder_recovery_max_bytes',
                value: String(bytes)
            });
        } catch {
            setError('Could not save the recovery storage budget.');
        } finally {
            setIsBusy(false);
        }
    };

    return (
        <div>
            <p class="setting-note">
                Folder archives are retained until restored. The storage budget blocks new deletions
                without erasing existing recovery data.
            </p>
            <label>
                Folder recovery budget (GiB)
                <Input
                    type="number"
                    min="1"
                    value={budget()}
                    onInput={event => setBudget(event.currentTarget.value)}
                    disabled={isBusy()}
                />
            </label>
            <Button variant="secondary" disabled={isBusy()} onClick={() => void saveBudget()}>
                Save Recovery Budget
            </Button>
            <Button variant="secondary" disabled={isBusy()} onClick={() => void load()}>
                Load Folder Recovery History
            </Button>
            <For each={entries()}>
                {entry => (
                    <div class="setting-action-row">
                        <span>
                            {entry.originalPath} · {entry.createdAt} ·{' '}
                            {entry.byteCount < 0
                                ? 'Inventory pending'
                                : (entry.byteCount / GIBIBYTE).toFixed(2) + ' GiB'}
                        </span>
                        <Show when={entry.restorable} fallback={<span>Restored</span>}>
                            <Button
                                variant="secondary"
                                disabled={isBusy()}
                                onClick={() => void restore(entry)}
                            >
                                Restore Folder
                            </Button>
                        </Show>
                    </div>
                )}
            </For>
            <Show when={hasMore()}>
                <Button disabled={isBusy()} onClick={() => void load(offset())}>
                    Load More
                </Button>
            </Show>
            <Show when={error()}>
                <p role="alert">{error()}</p>
            </Show>
        </div>
    );
};
