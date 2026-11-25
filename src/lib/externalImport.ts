/** Public HTTPS identity of the running local import API. */
export interface ExternalImportEndpoint {
    url: string;
    publicKeySha256: string;
    certificateDer: string;
}

/** Availability and current service state from the desktop. */
export interface ExternalImportStatus {
    available: boolean;
    endpoint: ExternalImportEndpoint | null;
}

/** Operations used by the integrations panel. */
export interface ExternalImportClient {
    status(): Promise<ExternalImportStatus>;
    start(): Promise<ExternalImportEndpoint>;
    stop(): Promise<void>;
    pairing: ExternalImportPairingClient;
}

/** Permission granted to a connected application. */
export type ImportScope = 'folders.read' | 'tags.read' | 'assets.import' | 'tags.assignOnImport';

/** Consent data visible only in the desktop. */
export interface ImportPairingStatus {
    catalogId: string;
    pending: { id: string; clientName: string; requestedScopes: ImportScope[] }[];
    clients: { id: string; clientName: string; scopes: ImportScope[]; rootIds: string[] }[];
    roots: { id: string; name: string; path: string }[];
}

/** Exact scope and root selection made by the user. */
export interface ImportApproval {
    requestId: string;
    catalogId: string;
    scopes: ImportScope[];
    rootIds: string[];
}

/** Trusted desktop operations for managing local clients. */
export interface ExternalImportPairingClient {
    copyInvitation(): Promise<void>;
    status(): Promise<ImportPairingStatus>;
    approve(approval: ImportApproval): Promise<void>;
    reject(requestId: string): Promise<void>;
    revoke(catalogId: string, clientId: string): Promise<void>;
}
