import { createSignal, onMount } from 'solid-js';
import { duplicatesApi, BackendDuplicateRuleSet } from '../../../../lib/duplicates';
import { DuplicateRuleProfile } from '../types';

export interface UseDuplicateRulesResult {
    selectedProfile: () => DuplicateRuleProfile;
    considerExactMatch: () => boolean;
    considerVisualMatch: () => boolean;
    considerCropMatch: () => boolean;
    ignoreResolutionDifference: () => boolean;
    ignoreRecompression: () => boolean;
    allowRotation: () => boolean;
    allowMirroring: () => boolean;
    minimumScore: () => number;
    minimumVisualScore: () => number;
    minimumCropScore: () => number;
    isLoading: () => boolean;
    isSaving: () => boolean;
    setConsiderExactMatch: (value: boolean) => void;
    setConsiderVisualMatch: (value: boolean) => void;
    setConsiderCropMatch: (value: boolean) => void;
    setIgnoreResolutionDifference: (value: boolean) => void;
    setIgnoreRecompression: (value: boolean) => void;
    setAllowRotation: (value: boolean) => void;
    setAllowMirroring: (value: boolean) => void;
    setMinimumScore: (value: number) => void;
    setMinimumVisualScore: (value: number) => void;
    setMinimumCropScore: (value: number) => void;
    handleProfileChange: (profileIdentifier: string) => void;
    hasUnsavedChanges: () => boolean;
    loadActiveRules: () => Promise<void>;
    saveRules: () => Promise<void>;
    saveAndRescan: () => Promise<void>;
}

function isExactMatchProfile(ruleSet: BackendDuplicateRuleSet): boolean {
    if (ruleSet.id === 'exact-match') {
        return true;
    }
    return (
        ruleSet.consider_exact_match &&
        !ruleSet.consider_visual_match &&
        !ruleSet.consider_crop_match
    );
}

function isAggressiveProfile(ruleSet: BackendDuplicateRuleSet): boolean {
    if (ruleSet.id === 'aggressive') {
        return true;
    }
    return (
        ruleSet.consider_exact_match && ruleSet.consider_visual_match && ruleSet.consider_crop_match
    );
}

function isVisualMatchProfile(ruleSet: BackendDuplicateRuleSet): boolean {
    if (ruleSet.id === 'visual-match') {
        return true;
    }
    return (
        !ruleSet.consider_exact_match &&
        ruleSet.consider_visual_match &&
        !ruleSet.consider_crop_match
    );
}

function deriveProfileFromRuleSet(ruleSet: BackendDuplicateRuleSet): DuplicateRuleProfile {
    if (isExactMatchProfile(ruleSet)) {
        return 'exact-match';
    }
    if (isAggressiveProfile(ruleSet)) {
        return 'aggressive';
    }
    if (isVisualMatchProfile(ruleSet)) {
        return 'visual-match';
    }
    return 'custom';
}

function checkRuleSetEquality(
    ruleSet: BackendDuplicateRuleSet | null,
    exactMatch: boolean,
    visualMatch: boolean,
    cropMatch: boolean,
    ignoreResolution: boolean,
    ignoreCompression: boolean,
    allowRotation: boolean,
    allowMirroring: boolean,
    similarityScore: number,
    visualScore: number,
    cropScore: number
): boolean {
    if (!ruleSet) {
        return true;
    }
    if (exactMatch !== ruleSet.consider_exact_match) {
        return false;
    }
    if (visualMatch !== ruleSet.consider_visual_match) {
        return false;
    }
    if (cropMatch !== ruleSet.consider_crop_match) {
        return false;
    }
    if (ignoreResolution !== ruleSet.ignore_resolution_difference) {
        return false;
    }
    if (ignoreCompression !== ruleSet.ignore_recompression) {
        return false;
    }
    if (allowRotation !== ruleSet.allow_rotation) {
        return false;
    }
    if (allowMirroring !== ruleSet.allow_mirroring) {
        return false;
    }
    if (Math.abs(similarityScore - ruleSet.min_score) > 0.001) {
        return false;
    }
    if (Math.abs(visualScore - ruleSet.min_visual_score) > 0.001) {
        return false;
    }
    if (Math.abs(cropScore - ruleSet.min_crop_score) > 0.001) {
        return false;
    }
    return true;
}

/**
 * Custom hook managing duplicate detection rules and algorithm configuration state.
 * Synchronizes with the active rule set persisted in the SQLite database.
 *
 * @returns {UseDuplicateRulesResult} Reactive signals and persistence actions for duplicate rules.
 */
export function useDuplicateRules(): UseDuplicateRulesResult {
    const [selectedProfile, setSelectedProfile] =
        createSignal<DuplicateRuleProfile>('visual-match');
    const [savedRuleSet, setSavedRuleSet] = createSignal<BackendDuplicateRuleSet | null>(null);
    const [considerExactMatch, setConsiderExactMatch] = createSignal<boolean>(false);
    const [considerVisualMatch, setConsiderVisualMatch] = createSignal<boolean>(true);
    const [considerCropMatch, setConsiderCropMatch] = createSignal<boolean>(false);
    const [ignoreResolutionDifference, setIgnoreResolutionDifference] = createSignal<boolean>(true);
    const [ignoreRecompression, setIgnoreRecompression] = createSignal<boolean>(true);
    const [allowRotation, setAllowRotation] = createSignal<boolean>(false);
    const [allowMirroring, setAllowMirroring] = createSignal<boolean>(false);
    const [minimumScore, setMinimumScore] = createSignal<number>(0.9);
    const [minimumVisualScore, setMinimumVisualScore] = createSignal<number>(0.9);
    const [minimumCropScore, setMinimumCropScore] = createSignal<number>(0.85);
    const [isLoading, setIsLoading] = createSignal<boolean>(false);
    const [isSaving, setIsSaving] = createSignal<boolean>(false);

    /**
     * Loads the active rule set from the database and updates state signals.
     */
    const loadActiveRules = async (): Promise<void> => {
        setIsLoading(true);
        try {
            const activeRuleSet = await duplicatesApi.getActiveDuplicateRuleSet();
            if (activeRuleSet) {
                setSavedRuleSet(activeRuleSet);
                const derivedProfile = deriveProfileFromRuleSet(activeRuleSet);
                setSelectedProfile(derivedProfile);
                setConsiderExactMatch(activeRuleSet.consider_exact_match);
                setConsiderVisualMatch(activeRuleSet.consider_visual_match);
                setConsiderCropMatch(activeRuleSet.consider_crop_match);
                setIgnoreResolutionDifference(activeRuleSet.ignore_resolution_difference);
                setIgnoreRecompression(activeRuleSet.ignore_recompression);
                setAllowRotation(activeRuleSet.allow_rotation);
                setAllowMirroring(activeRuleSet.allow_mirroring);
                setMinimumScore(activeRuleSet.min_score);
                setMinimumVisualScore(activeRuleSet.min_visual_score);
                setMinimumCropScore(activeRuleSet.min_crop_score);
            }
        } catch (error) {
            console.error('Failed to load active duplicate rules:', error);
        } finally {
            setIsLoading(false);
        }
    };

    /**
     * Handles switching between pre-calibrated profiles or custom configuration.
     *
     * @param {string} profileIdentifier - The chosen profile value.
     */
    const handleProfileChange = (profileIdentifier: string): void => {
        const normalizedProfile = profileIdentifier as DuplicateRuleProfile;
        setSelectedProfile(normalizedProfile);

        if (normalizedProfile === 'exact-match') {
            setConsiderExactMatch(true);
            setConsiderVisualMatch(false);
            setConsiderCropMatch(false);
            setIgnoreResolutionDifference(false);
            setIgnoreRecompression(false);
            setAllowRotation(false);
            setAllowMirroring(false);
            setMinimumScore(1.0);
            setMinimumVisualScore(1.0);
            setMinimumCropScore(1.0);
        } else if (normalizedProfile === 'visual-match') {
            setConsiderExactMatch(false);
            setConsiderVisualMatch(true);
            setConsiderCropMatch(false);
            setIgnoreResolutionDifference(true);
            setIgnoreRecompression(true);
            setAllowRotation(false);
            setAllowMirroring(false);
            setMinimumScore(0.9);
            setMinimumVisualScore(0.9);
            setMinimumCropScore(0.85);
        } else if (normalizedProfile === 'aggressive') {
            setConsiderExactMatch(true);
            setConsiderVisualMatch(true);
            setConsiderCropMatch(true);
            setIgnoreResolutionDifference(true);
            setIgnoreRecompression(true);
            setAllowRotation(true);
            setAllowMirroring(false);
            setMinimumScore(0.7);
            setMinimumVisualScore(0.7);
            setMinimumCropScore(0.7);
        }
    };

    /**
     * Persists the current rule set configuration to the database.
     */
    const saveRules = async (): Promise<void> => {
        setIsSaving(true);
        try {
            const profileIdentifier = selectedProfile();
            const ruleSetIdentifier =
                profileIdentifier === 'custom' ? 'custom-match' : profileIdentifier;

            const updatedRuleSet: BackendDuplicateRuleSet = {
                id: ruleSetIdentifier,
                name: profileIdentifier === 'custom' ? 'Custom Rules' : profileIdentifier,
                description: 'Configured duplicate detection rule set',
                consider_exact_match: considerExactMatch(),
                consider_visual_match: considerVisualMatch(),
                consider_crop_match: considerCropMatch(),
                ignore_resolution_difference: ignoreResolutionDifference(),
                ignore_recompression: ignoreRecompression(),
                allow_rotation: allowRotation(),
                allow_mirroring: allowMirroring(),
                min_score: minimumScore(),
                min_visual_score: minimumVisualScore(),
                min_crop_score: minimumCropScore(),
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString()
            };

            await duplicatesApi.updateDuplicateRuleSet(updatedRuleSet);
            setSavedRuleSet(updatedRuleSet);
        } catch (error) {
            console.error('Failed to save duplicate rule set:', error);
            throw error;
        } finally {
            setIsSaving(false);
        }
    };

    /**
     * Determines whether the current form state deviates from the persisted rule set.
     *
     * @returns {boolean} True if changes are pending save, false otherwise.
     */
    const hasUnsavedChanges = (): boolean => {
        return !checkRuleSetEquality(
            savedRuleSet(),
            considerExactMatch(),
            considerVisualMatch(),
            considerCropMatch(),
            ignoreResolutionDifference(),
            ignoreRecompression(),
            allowRotation(),
            allowMirroring(),
            minimumScore(),
            minimumVisualScore(),
            minimumCropScore()
        );
    };

    /**
     * Saves the current rule set and triggers a full library rescan.
     */
    const saveAndRescan = async (): Promise<void> => {
        await saveRules();
        await duplicatesApi.startDuplicateScan();
    };

    onMount(() => {
        void loadActiveRules();
    });

    return {
        selectedProfile,
        considerExactMatch,
        considerVisualMatch,
        considerCropMatch,
        ignoreResolutionDifference,
        ignoreRecompression,
        allowRotation,
        allowMirroring,
        minimumScore,
        minimumVisualScore,
        minimumCropScore,
        isLoading,
        isSaving,
        setConsiderExactMatch,
        setConsiderVisualMatch,
        setConsiderCropMatch,
        setIgnoreResolutionDifference,
        setIgnoreRecompression,
        setAllowRotation,
        setAllowMirroring,
        setMinimumScore,
        setMinimumVisualScore,
        setMinimumCropScore,
        handleProfileChange,
        hasUnsavedChanges,
        loadActiveRules,
        saveRules,
        saveAndRescan
    };
}
