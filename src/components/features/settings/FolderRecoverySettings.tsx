import { For, Show, createSignal, onMount, type Component } from 'solid-js';
import { Button, NumberInput } from '../../ui';
import { invokeCommand } from '../../../lib/api';
import { executeLibraryOperation } from '../../../core/store/library/folderActions';
import './folder-recovery-settings.css';

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

/**
 * Formats a byte count into a human-readable GiB string.
 *
 * @param {number} byteCount - The raw byte count.
 * @returns {string} A formatted size string.
 */
function formatEntryByteCount(byteCount: number): string {
    if (byteCount < 0) return 'Inventory pending';
    return (byteCount / GIBIBYTE).toFixed(2) + ' GiB';
}

/** Restores persisted Undo entries and controls storage growth without automatic deletion.
 * @returns Recovery controls for the trash settings section.
 * @example <FolderRecoverySettings />
 */
export const FolderRecoverySettings: Component = () => {
    const [entries, setEntries] = createSignal<RecoveryEntry[]>([]);
    const [listOffset, setListOffset] = createSignal(0);
    const [hasMoreEntries, setHasMoreEntries] = createSignal(false);
    const [isBusy, setIsBusy] = createSignal(false);
    const [errorMessage, setErrorMessage] = createSignal('');
    const [budgetGibibytes, setBudgetGibibytes] = createSignal(50);

    onMount(async () => {
        try {
            const storedValue = await invokeCommand<string | null>('get_setting', {
                key: 'folder_recovery_max_bytes'
            });
            if (storedValue && Number(storedValue) > 0) {
                setBudgetGibibytes(Number(storedValue) / GIBIBYTE);
            }
        } catch {
            setErrorMessage('Could not load the recovery storage budget.');
        }
    });

    const loadEntries = async (pageOffset = 0): Promise<void> => {
        if (isBusy()) return;
        setIsBusy(true);
        setErrorMessage('');
        try {
            const entryPage = await invokeCommand<RecoveryEntry[]>('folder_recovery_history', {
                offset: pageOffset
            });
            setEntries(pageOffset ? previous => [...previous, ...entryPage] : entryPage);
            setListOffset(pageOffset + entryPage.length);
            setHasMoreEntries(entryPage.length === 50);
        } catch {
            setErrorMessage('Could not load folder recovery history.');
        } finally {
            setIsBusy(false);
        }
    };

    const restoreFolder = async (entry: RecoveryEntry): Promise<void> => {
        if (isBusy()) return;
        setIsBusy(true);
        setErrorMessage('');
        try {
            const result = await executeLibraryOperation({
                kind: 'restoreFolder',
                folderId: entry.folderId
            });
            setEntries(previous =>
                previous.filter(candidate => candidate.folderId !== entry.folderId)
            );
            setListOffset(previous => Math.max(0, previous - 1));
            if (result.warning) setErrorMessage(result.warning);
        } catch (failure) {
            setErrorMessage(
                typeof failure === 'object' && failure && 'message' in failure
                    ? String(failure.message)
                    : 'Restore failed. The archive is retained for retry.'
            );
        } finally {
            setIsBusy(false);
        }
    };

    const saveBudget = async (): Promise<void> => {
        const byteBudget = Math.floor(budgetGibibytes() * GIBIBYTE);
        if (!Number.isSafeInteger(byteBudget) || byteBudget <= 0) {
            setErrorMessage('Enter a positive storage budget.');
            return;
        }
        setIsBusy(true);
        setErrorMessage('');
        try {
            await invokeCommand('set_setting', {
                key: 'folder_recovery_max_bytes',
                value: String(byteBudget)
            });
        } catch {
            setErrorMessage('Could not save the recovery storage budget.');
        } finally {
            setIsBusy(false);
        }
    };

    return (
        <div class="folder-recovery-section">
            <div class="general-setting-row">
                <div class="general-setting-text-block">
                    <span class="setting-label">Recovery Storage Budget</span>
                    <span class="general-setting-sublabel">
                        Maximum disk space reserved for deleted folder archives (GiB). Existing
                        archives are never erased to enforce this limit.
                    </span>
                </div>
                <div class="folder-recovery-budget-input">
                    <NumberInput
                        min={1}
                        aria-label="Folder recovery budget (GiB)"
                        value={budgetGibibytes()}
                        onChange={value => setBudgetGibibytes(value ?? 1)}
                        disabled={isBusy()}
                    />
                </div>
            </div>

            <div class="storage-actions-row">
                <Button variant="secondary" disabled={isBusy()} onClick={() => void saveBudget()}>
                    Save Recovery Budget
                </Button>
                <Button variant="secondary" disabled={isBusy()} onClick={() => void loadEntries()}>
                    Load Folder Recovery History
                </Button>
            </div>

            <Show when={entries().length > 0}>
                <div class="folder-recovery-list">
                    <For each={entries()}>
                        {entry => (
                            <div class="folder-recovery-entry">
                                <div class="folder-recovery-entry-info">
                                    <span
                                        class="folder-recovery-entry-path"
                                        title={entry.originalPath}
                                    >
                                        {entry.originalPath}
                                    </span>
                                    <span class="folder-recovery-entry-meta">
                                        {entry.createdAt} · {formatEntryByteCount(entry.byteCount)}
                                    </span>
                                </div>
                                <Show
                                    when={entry.restorable}
                                    fallback={
                                        <span class="folder-recovery-entry-restored">Restored</span>
                                    }
                                >
                                    <Button
                                        variant="secondary"
                                        size="sm"
                                        disabled={isBusy()}
                                        onClick={() => void restoreFolder(entry)}
                                    >
                                        Restore Folder
                                    </Button>
                                </Show>
                            </div>
                        )}
                    </For>
                </div>
            </Show>

            <Show when={hasMoreEntries()}>
                <div class="folder-recovery-load-more">
                    <Button
                        variant="secondary"
                        size="sm"
                        disabled={isBusy()}
                        onClick={() => void loadEntries(listOffset())}
                    >
                        Load More
                    </Button>
                </div>
            </Show>

            <Show when={errorMessage()}>
                <p role="alert" class="folder-recovery-error">
                    {errorMessage()}
                </p>
            </Show>
        </div>
    );
};
