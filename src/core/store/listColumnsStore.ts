import { createRoot, createSignal } from 'solid-js';

/**
 * Storage key for list view column configurations
 */
const COLUMN_STORAGE_KEY = 'mundam-viewport-columns-v1';

/**
 * Persisted configuration of a list view column
 */
export interface ListColumnConfig {
    width?: number;
    hidden?: boolean;
}

/**
 * Columns that the user can show or hide in the list view
 */
export const TOGGLEABLE_LIST_COLUMNS: { key: string; label: string; defaultHidden: boolean }[] = [
    { key: 'rating', label: 'Rating', defaultHidden: false },
    { key: 'format', label: 'Type', defaultHidden: false },
    { key: 'size', label: 'Size', defaultHidden: false },
    { key: 'width', label: 'Dimensions', defaultHidden: false },
    { key: 'created_at', label: 'Created', defaultHidden: true },
    { key: 'modified_at', label: 'Modified', defaultHidden: true },
    { key: 'added_at', label: 'Added', defaultHidden: true }
];

const loadSavedConfigs = (): Record<string, ListColumnConfig> => {
    try {
        const saved = localStorage.getItem(COLUMN_STORAGE_KEY);
        return saved ? JSON.parse(saved) : {};
    } catch {
        return {};
    }
};

/**
 * Single source of truth for list view columns, shared by the table
 * (header context menu / resize) and the view options popover.
 */
const store = createRoot(() => {
    const [columnConfigs, setColumnConfigs] =
        createSignal<Record<string, ListColumnConfig>>(loadSavedConfigs());

    const updateColumnConfig = (key: string, updates: ListColumnConfig) => {
        setColumnConfigs(previous => {
            const next = { ...previous, [key]: { ...previous[key], ...updates } };
            localStorage.setItem(COLUMN_STORAGE_KEY, JSON.stringify(next));
            return next;
        });
    };

    return { columnConfigs, updateColumnConfig };
});

export const listColumnConfigs = store.columnConfigs;
export const updateListColumnConfig = store.updateColumnConfig;

/**
 * Whether a column is hidden, falling back to its default.
 */
export const isListColumnHidden = (key: string, defaultHidden: boolean): boolean =>
    listColumnConfigs()[key]?.hidden ?? defaultHidden;
