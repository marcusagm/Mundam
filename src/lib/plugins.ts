/** Profile-wide plugin administration contract consumed by Settings. */
export interface PluginSettingsClient {
    load(): Promise<PluginSettings>;
    /** Returns the number of saved annotations, or null when the file dialog is canceled. */
    exportAnnotations(): Promise<number | null>;
    storedData: PluginStoredDataClient;
    trust(namespace: string, fingerprint: string): Promise<void>;
    revoke(namespace: string, fingerprint: string): Promise<void>;
    choose(): Promise<PluginPackageReview | null>;
    install(reviewId: string): Promise<InstalledPlugin>;
    discard(reviewId: string): Promise<void>;
    reviewActivation(pluginId: string): Promise<PluginActivationReview>;
    enable(reviewId: string, optionalPermissions: PluginPermission[]): Promise<void>;
    disable(pluginId: string): Promise<void>;
    reviewRollback(pluginId: string): Promise<PluginPackageReview>;
    remove(pluginId: string, packageRevision: string): Promise<void>;
}

/** The plugin ID and signing identity that own catalog data. */
export interface StoredPluginOwner {
    pluginId: string;
    publisherFingerprint: string;
}

/** Counts of durable values and regenerable results in the active catalog. */
export interface StoredPluginDataCounts {
    annotations: number;
    recoveryAnnotations: number;
    cachedResults: number;
}

/** Settings access to data retained independently of package installation. */
export interface PluginStoredDataClient {
    list(): Promise<{ owner: StoredPluginOwner; counts: StoredPluginDataCounts }[]>;
    review(owner: StoredPluginOwner): Promise<{ counts: StoredPluginDataCounts; revision: string }>;
    purge(owner: StoredPluginOwner, revision: string): Promise<boolean>;
}

/** Recorded publisher identity and revocation status. */
export interface PluginPublisher {
    namespace: string;
    fingerprint: string;
    revoked: boolean;
}

/** Current state of an installed package. */
export interface InstalledPlugin {
    id: string;
    publisher: string;
    name: string;
    version: string;
    enabled: boolean;
    approved: boolean;
    packageRevision: string;
    rollbackVersion: string | null;
}

/** Metadata-only settings inventory. */
export interface PluginSettings {
    publishers: PluginPublisher[];
    installed: InstalledPlugin[];
    safeMode: boolean;
}

/** Permissions that the current declarative SDK can request. */
export type PluginPermission =
    | 'selection.read'
    | 'assetPreview.read'
    | 'assetData.own.read'
    | 'assetData.own.write';

/** Permission request for one installed package in the current catalog. */
export interface PluginActivationReview {
    reviewId: string;
    name: string;
    version: string;
    requiredPermissions: PluginPermission[];
    optionalPermissions: PluginPermission[];
    experimentalWorkerId?: string;
}

/** Verified package details with a one-time review identifier if compatible. */
export interface PluginPackageReview {
    reviewId: string | null;
    pluginId: string;
    name: string;
    description: string;
    version: string;
    license: string;
    publisher: string;
    publisherFingerprint: string;
    iconBase64: string;
    targets: { platform: string; architecture: string }[];
    requiredPermissions: PluginPermission[];
    optionalPermissions: PluginPermission[];
    experimentalWorkerId?: string;
    compatible: boolean;
    replacement: PluginReplacementReview | null;
}

/** Describes a verified update or rollback relative to the installed version. */
export interface PluginReplacementReview {
    kind: 'update' | 'rollback';
    currentVersion: string;
    addedPermissions: PluginPermission[];
    removedPermissions: PluginPermission[];
}
