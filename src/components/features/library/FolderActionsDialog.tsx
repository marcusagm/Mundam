import { Component, Show, createEffect, createMemo, createSignal } from 'solid-js';
import { Modal, Button, Input } from '../../ui';
import { useMetadata } from '../../../core/hooks';
import {
    executeLibraryOperation,
    type LibraryOperation
} from '../../../core/store/library/folderActions';
import { FolderDestinationPicker } from './FolderDestinationPicker';
import { showFolderDeletionUndo } from './folderDeletionUndo';
import { toast } from '../../ui/Sonner/state';
import './folder-actions-dialog.css';

/** Supported folder dialogs, with a shared destination picker for assets. */
export type FolderDialogAction = 'moveFolder' | 'deleteFolder' | 'moveAssets' | 'renameAsset';

/** Dialog target is captured before the context menu closes. */
export type FolderDialogTarget =
    | {
          action: 'moveFolder' | 'deleteFolder';
          folderId: string;
          assetIds?: never;
          initialName?: never;
      }
    | { action: 'moveAssets'; assetIds: string[]; folderId?: never; initialName?: never }
    | { action: 'renameAsset'; assetIds: [string]; initialName: string; folderId?: never };

/** Controls one explicit managed-library intention. */
export interface FolderActionsDialogProps {
    /** Null closes the dialog; each action requires its relevant identifiers. */
    target: FolderDialogTarget | null;
    /** Runs after commit or explicit cancellation. */
    onClose: () => void;
}

/** Collects input while keeping failed operations reviewable.
 * @param props - Discriminated target and close callback.
 * @returns Shared modal with destination tree or rename input.
 * @example <FolderActionsDialog target={null} onClose={() => {}} />
 */
export const FolderActionsDialog: Component<FolderActionsDialogProps> = props => {
    const metadata = useMetadata();
    const [name, setName] = createSignal('');
    const [destinationId, setDestinationId] = createSignal('');
    const [isBusy, setIsBusy] = createSignal(false);
    const [errorMessage, setErrorMessage] = createSignal('');
    const folder = createMemo(() =>
        metadata.locations.find(folder => folder.id === props.target?.folderId)
    );
    const action = () => props.target?.action;
    const isMove = () => action() === 'moveFolder' || action() === 'moveAssets';
    const titles: Record<FolderDialogAction, string> = {
        moveFolder: 'Move Folder',
        deleteFolder: 'Delete Folder',
        moveAssets: 'Move Assets',
        renameAsset: 'Rename Asset'
    };
    createEffect(() => {
        const target = props.target;
        if (!target) return;
        setName(target.initialName || '');
        setDestinationId('');
        setErrorMessage('');
    });

    /** Builds the explicit backend intention from the currently visible controls. */
    const operation = (): LibraryOperation => {
        const target = props.target;
        if (!target) throw new Error('No active library operation');
        switch (target.action) {
            case 'renameAsset':
                return { kind: 'renameAsset', assetId: target.assetIds[0], name: name() };
            case 'moveFolder':
                return {
                    kind: 'moveFolder',
                    folderId: target.folderId,
                    destinationId: destinationId()
                };
            case 'moveAssets':
                return {
                    kind: 'moveAssets',
                    assetIds: target.assetIds,
                    destinationId: destinationId()
                };
            case 'deleteFolder':
                return { kind: 'deleteFolder', folderId: target.folderId };
        }
    };

    /** Leaves the dialog open on failure and prevents overlapping submissions. */
    const handleSubmit = async (event: SubmitEvent): Promise<void> => {
        event.preventDefault();
        if (isBusy()) return;
        if (isMove() && !destinationId()) {
            setErrorMessage('Choose a destination folder.');
            return;
        }
        setIsBusy(true);
        setErrorMessage('');
        try {
            const intention = operation();
            const result = await executeLibraryOperation(intention);
            if (intention.kind === 'deleteFolder') {
                showFolderDeletionUndo(result.id);
            } else toast.success('Library updated');
            if (result.warning) toast.warning('Folder monitoring', { description: result.warning });
            props.onClose();
        } catch (error) {
            setErrorMessage(
                typeof error === 'object' && error && 'message' in error
                    ? String(error.message)
                    : String(error)
            );
        } finally {
            setIsBusy(false);
        }
    };

    return (
        <Modal
            isOpen={!!props.target}
            onClose={() => {
                if (!isBusy()) props.onClose();
            }}
            title={titles[action() || 'moveFolder']}
        >
            <form class="folder-actions-form" onSubmit={handleSubmit}>
                <Show when={action() === 'renameAsset'}>
                    <label>
                        Name
                        <Input
                            value={name()}
                            onInput={event => setName(event.currentTarget.value)}
                            onKeyDown={event => {
                                if (event.key === 'Enter')
                                    event.currentTarget.form?.requestSubmit();
                            }}
                            required
                            disabled={isBusy()}
                        />
                    </label>
                </Show>
                <Show when={isMove()}>
                    <FolderDestinationPicker
                        value={destinationId()}
                        onSelect={setDestinationId}
                        excludedFolderId={
                            action() === 'moveFolder' ? props.target?.folderId : undefined
                        }
                        isDisabled={isBusy()}
                    />
                </Show>
                <Show when={action() === 'deleteFolder'}>
                    <p>
                        Delete “{folder()?.name}” and all its contents from the library? Original
                        files remain recoverable. Use Undo after deletion to restore this folder,
                        its files and library metadata.
                    </p>
                </Show>
                <Show when={errorMessage()}>
                    <p role="alert">{errorMessage()}</p>
                </Show>
                <div class="folder-actions-buttons">
                    <Button
                        type="button"
                        variant="ghost"
                        disabled={isBusy()}
                        onClick={props.onClose}
                    >
                        Cancel
                    </Button>
                    <Button
                        type="submit"
                        variant={action() === 'deleteFolder' ? 'destructive' : 'primary'}
                        disabled={isBusy() || (isMove() && !destinationId())}
                    >
                        {isBusy() ? 'Working…' : 'Confirm'}
                    </Button>
                </div>
            </form>
        </Modal>
    );
};
