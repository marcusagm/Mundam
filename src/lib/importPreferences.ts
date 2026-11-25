/** Persisted import destination in the current catalog. */
export interface ImportPreference {
    catalogId: string;
    folderId: string | null;
}

/** Operations used by the settings panel, independent of the desktop transport. */
export interface ImportPreferenceClient {
    load(): Promise<ImportPreference>;
    save(catalogId: string, folderId: string | null): Promise<ImportPreference>;
}
