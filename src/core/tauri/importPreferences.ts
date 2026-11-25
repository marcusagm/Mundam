import type { ImportPreferenceClient } from '../../lib/importPreferences';
import { invokeCommand } from '../../lib/api';

/** Connects the import preference interface to the desktop commands. */
export const desktopImportPreferences: ImportPreferenceClient = {
    load: () => invokeCommand('get_import_preference'),
    save: (catalogId, folderId) => invokeCommand('set_import_preference', { catalogId, folderId })
};
