import { setMetadataState, type SmartFolder } from './metadataState';
import { type SearchGroup } from '../filter';
import { ActionResult, ErrorCode } from '../../types/actions';
import { invokeCommand as invoke } from '../../../lib/api';

export const searchActions = {
    loadSmartFolders: async (failOnError = false) => {
        try {
            const folders = (await invoke('get_smart_folders')) as SmartFolder[];
            setMetadataState('smartFolders', folders);
        } catch (error) {
            console.error('Failed to load smart folders:', error);
            if (failOnError) throw error;
        }
    },

    saveSmartFolder: async (
        name: string,
        query: SearchGroup | null,
        id?: string
    ): Promise<ActionResult> => {
        try {
            if (id) {
                await invoke('update_smart_folder', {
                    id: String(id),
                    name,
                    query: JSON.stringify(query)
                });
            } else {
                await invoke('save_smart_folder', { name, query: JSON.stringify(query) });
            }
            await searchActions.loadSmartFolders();
            return { success: true, data: undefined };
        } catch (error) {
            console.error('Failed to save smart folder:', error);
            return {
                success: false,
                error: {
                    code: ErrorCode.IO_ERROR,
                    message: 'Failed to save smart folder'
                }
            };
        }
    },

    deleteSmartFolder: async (id: string): Promise<ActionResult> => {
        try {
            await invoke('delete_smart_folder', { id: String(id) });
            await searchActions.loadSmartFolders();
            return { success: true, data: undefined };
        } catch (error) {
            console.error('Failed to delete smart folder:', error);
            return {
                success: false,
                error: {
                    code: ErrorCode.IO_ERROR,
                    message: 'Failed to delete smart folder'
                }
            };
        }
    }
};
