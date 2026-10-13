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
    minimumScore: () => number;
    isLoading: () => boolean;
    isSaving: () => boolean;
    setConsiderExactMatch: (value: boolean) => void;
    setConsiderVisualMatch: (value: boolean) => void;
    setConsiderCropMatch: (value: boolean) => void;
    setIgnoreResolutionDifference: (value: boolean) => void;
    setIgnoreRecompression: (value: boolean) => void;
    setMinimumScore: (value: number) => void;
    handleProfileChange: (profileIdentifier: string) => void;
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

/**
 * Custom hook managing duplicate detection rules and algorithm configuration state.
 * Synchronizes with the active rule set persisted in the SQLite database.
 *
 * @returns {UseDuplicateRulesResult} Reactive signals and persistence actions for duplicate rules.
 */
export function useDuplicateRules(): UseDuplicateRulesResult {
    const [selectedProfile, setSelectedProfile] =
        createSignal<DuplicateRuleProfile>('visual-match');
    const [considerExactMatch, setConsiderExactMatch] = createSignal<boolean>(false);
    const [considerVisualMatch, setConsiderVisualMatch] = createSignal<boolean>(true);
    const [considerCropMatch, setConsiderCropMatch] = createSignal<boolean>(false);
    const [ignoreResolutionDifference, setIgnoreResolutionDifference] = createSignal<boolean>(true);
    const [ignoreRecompression, setIgnoreRecompression] = createSignal<boolean>(true);
    const [minimumScore, setMinimumScore] = createSignal<number>(0.9);
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
                const derivedProfile = deriveProfileFromRuleSet(activeRuleSet);
                setSelectedProfile(derivedProfile);
                setConsiderExactMatch(activeRuleSet.consider_exact_match);
                setConsiderVisualMatch(activeRuleSet.consider_visual_match);
                setConsiderCropMatch(activeRuleSet.consider_crop_match);
                setIgnoreResolutionDifference(activeRuleSet.ignore_resolution_difference);
                setIgnoreRecompression(activeRuleSet.ignore_recompression);
                setMinimumScore(activeRuleSet.min_score);
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
            setMinimumScore(1.0);
        } else if (normalizedProfile === 'visual-match') {
            setConsiderExactMatch(false);
            setConsiderVisualMatch(true);
            setConsiderCropMatch(false);
            setIgnoreResolutionDifference(true);
            setIgnoreRecompression(true);
            setMinimumScore(0.9);
        } else if (normalizedProfile === 'aggressive') {
            setConsiderExactMatch(true);
            setConsiderVisualMatch(true);
            setConsiderCropMatch(true);
            setIgnoreResolutionDifference(true);
            setIgnoreRecompression(true);
            setMinimumScore(0.7);
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

            await duplicatesApi.updateDuplicateRuleSet({
                id: ruleSetIdentifier,
                name: profileIdentifier === 'custom' ? 'Custom Rules' : profileIdentifier,
                description: 'Configured duplicate detection rule set',
                consider_exact_match: considerExactMatch(),
                consider_visual_match: considerVisualMatch(),
                consider_crop_match: considerCropMatch(),
                ignore_resolution_difference: ignoreResolutionDifference(),
                ignore_recompression: ignoreRecompression(),
                allow_rotation: false,
                allow_mirroring: false,
                min_score: minimumScore(),
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString()
            });
        } catch (error) {
            console.error('Failed to save duplicate rule set:', error);
            throw error;
        } finally {
            setIsSaving(false);
        }
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
        minimumScore,
        isLoading,
        isSaving,
        setConsiderExactMatch,
        setConsiderVisualMatch,
        setConsiderCropMatch,
        setIgnoreResolutionDifference,
        setIgnoreRecompression,
        setMinimumScore,
        handleProfileChange,
        loadActiveRules,
        saveRules,
        saveAndRescan
    };
}
