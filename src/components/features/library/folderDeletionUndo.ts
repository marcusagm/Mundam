import { toast } from '../../ui/Sonner/state';
import { executeLibraryOperation } from '../../../core/store/library/folderActions';

/** Offers a durable undo action without exposing recovery storage in routine flows. */
export function showFolderDeletionUndo(folderId: string): void {
    let isRestoring = false;
    const undo = async (): Promise<void> => {
        if (isRestoring) return;
        isRestoring = true;
        try {
            await executeLibraryOperation({ kind: 'restoreFolder', folderId });
            toast.success('Folder restored');
        } catch {
            toast.error('Could not undo folder deletion', {
                description:
                    'The recovery copy is preserved. Check that the original location is available and empty.',
                duration: 0,
                action: {
                    label: 'Try Undo Again',
                    onClick: () => {
                        void undo();
                    }
                }
            });
        } finally {
            isRestoring = false;
        }
    };
    toast.success('Folder deleted', {
        description: 'Undo restores its files, subfolders and library metadata.',
        duration: 0,
        action: {
            label: 'Undo',
            onClick: () => {
                void undo();
            }
        }
    });
}
