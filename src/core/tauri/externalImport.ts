import { invokeCommand } from '../../lib/api';
import type { ExternalImportClient } from '../../lib/externalImport';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';

/** Desktop adapter for local import API controls. */
export const desktopExternalImport: ExternalImportClient = {
    status: () => invokeCommand('get_external_import_status'),
    start: () => invokeCommand('start_external_import'),
    stop: () => invokeCommand('stop_external_import'),
    pairing: {
        copyInvitation: async () => {
            const invitation = await invokeCommand('create_external_import_invitation');
            await writeText(JSON.stringify(invitation));
        },
        status: () => invokeCommand('list_external_import_pairings'),
        approve: approval =>
            invokeCommand('approve_external_import_pairing', {
                requestId: approval.requestId,
                catalogId: approval.catalogId,
                scopes: approval.scopes,
                rootIds: approval.rootIds
            }),
        reject: requestId => invokeCommand('reject_external_import_pairing', { requestId }),
        revoke: (catalogId, clientId) =>
            invokeCommand('revoke_external_import_client', { catalogId, clientId })
    }
};
