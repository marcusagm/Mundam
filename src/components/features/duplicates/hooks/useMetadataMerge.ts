import { createSignal, Accessor } from 'solid-js';
import { invoke } from '@tauri-apps/api/core';
import { DuplicateCandidate } from '../types';
import { MetadataMergePayload, duplicatesApi } from '../../../../lib/duplicates';

/**
 * A single EXIF/technical field entry coming from one candidate's technical payload.
 */
export interface TechnicalFieldEntry {
    /** The field name (e.g. 'Artist', 'Copyright', 'GPSLatitude'). */
    fieldName: string;
    /**
     * Each candidate's value for this field.
     * The key is the asset ID, the value is the stringified field value.
     */
    valuesByAssetId: Record<string, string>;
}

/**
 * The user's current selection for a divergent technical field.
 * `null` means the user chose to skip this field.
 */
export interface TechnicalFieldChoice {
    /** The asset ID whose value the user selected as the source, or null to skip. */
    chosenAssetId: string | null;
}

/**
 * Return type of the `useMetadataMerge` hook.
 */
export interface UseMetadataMergeReturn {
    /** Whether the backend EXIF data is currently loading. */
    isLoadingExif: Accessor<boolean>;
    /** Divergent technical fields found across all candidates. */
    divergentTechnicalFields: Accessor<TechnicalFieldEntry[]>;
    /** The user's current choice for each technical field. */
    technicalFieldChoices: Accessor<Record<string, TechnicalFieldChoice>>;
    /** The current rating value the user confirmed (from 0 to 5). */
    selectedRating: Accessor<number | null>;
    /** The current favorite state the user confirmed. */
    selectedIsFavorite: Accessor<boolean | null>;
    /** The current notes content the user confirmed or edited. */
    selectedNotes: Accessor<string | null>;
    /** Tag IDs the user selected for merging into kept assets. */
    selectedTagIds: Accessor<Set<string>>;
    suggestedRating: number;
    suggestedIsFavorite: boolean;
    suggestedNotes: string | null;
    discardedTagIds: Set<string>;
    /** Updates the user's rating choice. */
    setSelectedRating: (rating: number | null) => void;
    /** Updates the user's favorite choice. */
    setSelectedIsFavorite: (isFavorite: boolean | null) => void;
    /** Updates the user's notes content. */
    setSelectedNotes: (notes: string | null) => void;
    /** Toggles a tag ID in the selection set. */
    toggleTagId: (tagId: string) => void;
    /** Updates the user's choice for a divergent technical field. */
    setTechnicalFieldChoice: (fieldName: string, choice: TechnicalFieldChoice) => void;
    /** Builds the final MetadataMergePayload from the current user selections. */
    buildMergePayload: () => MetadataMergePayload;
    /** Applies the confirmed merge and resolves the group. Returns after both succeed. */
    applyMergeAndResolve: (
        groupId: string,
        keptAssetIds: string[],
        resolveFn: (groupId: string, action: string, keptAssetIds?: string[]) => Promise<void>
    ) => Promise<void>;
    /** Whether an apply operation is in progress. */
    isApplying: Accessor<boolean>;
}

/**
 * Fetches the technical (EXIF/XMP/IPTC) payload for a single asset.
 *
 * @param {string} assetId - The asset ID to fetch EXIF data for.
 * @returns {Promise<Record<string, string>>} The flat EXIF key-value map.
 */
async function fetchAssetExif(assetId: string): Promise<Record<string, string>> {
    try {
        const result = await invoke<Record<string, string>>('get_asset_exif', { assetId });
        return result ?? {};
    } catch {
        return {};
    }
}

/**
 * Finds technical fields that differ across at least two candidates.
 *
 * @param {Record<string, Record<string, string>>} exifByAssetId - EXIF map keyed by asset ID.
 * @returns {TechnicalFieldEntry[]} The divergent fields with per-candidate values.
 */
function findDivergentFields(
    exifByAssetId: Record<string, Record<string, string>>
): TechnicalFieldEntry[] {
    const assetIds = Object.keys(exifByAssetId);
    if (assetIds.length < 2) return [];

    const allFieldNames = new Set<string>(
        assetIds.flatMap(assetId => Object.keys(exifByAssetId[assetId]))
    );

    const divergentEntries: TechnicalFieldEntry[] = [];

    for (const fieldName of allFieldNames) {
        const valuesByAssetId: Record<string, string> = {};
        const uniqueValues = new Set<string>();

        for (const assetId of assetIds) {
            const value = exifByAssetId[assetId][fieldName];
            if (value !== undefined && value !== null && value !== '') {
                valuesByAssetId[assetId] = value;
                uniqueValues.add(value);
            }
        }

        if (uniqueValues.size > 1) {
            divergentEntries.push({ fieldName, valuesByAssetId });
        }
    }

    return divergentEntries;
}

/**
 * Hook that manages the complete metadata merge flow for a duplicate group.
 * Fetches EXIF data from all candidates, identifies divergent fields,
 * and maintains the user's field-by-field confirmation state.
 *
 * @param {DuplicateCandidate[]} candidates - All candidates in the group (kept + discarded).
 * @param {DuplicateCandidate[]} keptCandidates - The candidates the user chose to keep.
 * @param {DuplicateCandidate[]} discardedCandidates - The candidates that will be discarded.
 * @returns {UseMetadataMergeReturn} State and actions for the merge UI.
 */
export function useMetadataMerge(
    candidates: DuplicateCandidate[],
    _keptCandidates: DuplicateCandidate[],
    discardedCandidates: DuplicateCandidate[]
): UseMetadataMergeReturn {
    const [isLoadingExif, setIsLoadingExif] = createSignal(true);
    const [isApplying, setIsApplying] = createSignal(false);
    const [divergentTechnicalFields, setDivergentTechnicalFields] = createSignal<
        TechnicalFieldEntry[]
    >([]);
    const [technicalFieldChoices, setTechnicalFieldChoices] = createSignal<
        Record<string, TechnicalFieldChoice>
    >({});

    const suggestedRating = candidates.reduce((maximum, candidate) => {
        const candidateRating = (candidate as DuplicateCandidate & { rating?: number }).rating ?? 0;
        return Math.max(maximum, candidateRating);
    }, 0);

    const suggestedIsFavorite = candidates.some(candidate => candidate.isFavorite);

    const discardedNotesList = discardedCandidates
        .map(candidate => (candidate as DuplicateCandidate & { notes?: string }).notes)
        .filter((note): note is string => Boolean(note));
    const suggestedNotes =
        discardedNotesList.length > 0 ? discardedNotesList.join('\n---\n') : null;

    const discardedTagIds = new Set<string>(
        discardedCandidates.flatMap(candidate => candidate.tags.map(tag => tag.id))
    );

    const [selectedRating, setSelectedRating] = createSignal<number | null>(
        suggestedRating > 0 ? suggestedRating : null
    );
    const [selectedIsFavorite, setSelectedIsFavorite] = createSignal<boolean | null>(
        suggestedIsFavorite ? true : null
    );
    const [selectedNotes, setSelectedNotes] = createSignal<string | null>(suggestedNotes);
    const [selectedTagIds, setSelectedTagIds] = createSignal<Set<string>>(new Set(discardedTagIds));

    const toggleTagId = (tagId: string): void => {
        const currentSet = new Set(selectedTagIds());
        if (currentSet.has(tagId)) {
            currentSet.delete(tagId);
        } else {
            currentSet.add(tagId);
        }
        setSelectedTagIds(currentSet);
    };

    const setTechnicalFieldChoice = (fieldName: string, choice: TechnicalFieldChoice): void => {
        setTechnicalFieldChoices(previous => ({ ...previous, [fieldName]: choice }));
    };

    const buildMergePayload = (): MetadataMergePayload => {
        const choices = technicalFieldChoices();
        const fields = divergentTechnicalFields();

        const technicalPayloadOverride: Record<string, unknown> = {};
        let hasTechnicalOverride = false;

        for (const field of fields) {
            const choice = choices[field.fieldName];
            if (choice?.chosenAssetId && field.valuesByAssetId[choice.chosenAssetId]) {
                technicalPayloadOverride[field.fieldName] =
                    field.valuesByAssetId[choice.chosenAssetId];
                hasTechnicalOverride = true;
            }
        }

        return {
            rating: selectedRating(),
            isFavorite: selectedIsFavorite(),
            notes: selectedNotes(),
            tagsToAdd: Array.from(selectedTagIds()),
            technicalPayloadOverride: hasTechnicalOverride ? technicalPayloadOverride : null
        };
    };

    const applyMergeAndResolve = async (
        groupId: string,
        keptAssetIds: string[],
        resolveFn: (groupId: string, action: string, keptAssetIds?: string[]) => Promise<void>
    ): Promise<void> => {
        setIsApplying(true);
        try {
            const mergePayload = buildMergePayload();
            const hasMergeData =
                mergePayload.rating !== null ||
                mergePayload.isFavorite !== null ||
                mergePayload.notes !== null ||
                mergePayload.tagsToAdd.length > 0 ||
                mergePayload.technicalPayloadOverride !== null;

            if (hasMergeData) {
                await duplicatesApi.applyMetadataMerge(keptAssetIds, mergePayload);
            }
            await resolveFn(groupId, 'custom_selection', keptAssetIds);
        } finally {
            setIsApplying(false);
        }
    };

    (async () => {
        const exifByAssetId: Record<string, Record<string, string>> = {};

        await Promise.all(
            candidates.map(async candidate => {
                exifByAssetId[candidate.id] = await fetchAssetExif(candidate.id);
            })
        );

        const divergent = findDivergentFields(exifByAssetId);
        setDivergentTechnicalFields(divergent);

        const initialChoices: Record<string, TechnicalFieldChoice> = {};
        for (const field of divergent) {
            const firstDiscardedId =
                discardedCandidates.find(
                    discarded => field.valuesByAssetId[discarded.id] !== undefined
                )?.id ?? null;
            initialChoices[field.fieldName] = { chosenAssetId: firstDiscardedId };
        }
        setTechnicalFieldChoices(initialChoices);
        setIsLoadingExif(false);
    })();

    return {
        isLoadingExif,
        divergentTechnicalFields,
        technicalFieldChoices,
        suggestedRating,
        suggestedIsFavorite,
        suggestedNotes,
        discardedTagIds,
        selectedRating,
        selectedIsFavorite,
        selectedNotes,
        selectedTagIds,
        setSelectedRating,
        setSelectedIsFavorite,
        setSelectedNotes,
        toggleTagId,
        setTechnicalFieldChoice,
        buildMergePayload,
        applyMergeAndResolve,
        isApplying
    };
}
