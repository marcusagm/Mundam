import { BackendDuplicateRuleSet } from '../../../../lib/duplicates';
import { DuplicateRuleProfile } from '../types';

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

export function deriveProfileFromRuleSet(ruleSet: BackendDuplicateRuleSet): DuplicateRuleProfile {
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

function checkRuleFlagsEquality(ruleSet: BackendDuplicateRuleSet, flags: boolean[]): boolean {
    const targetFlags = [
        ruleSet.consider_exact_match,
        ruleSet.consider_visual_match,
        ruleSet.consider_crop_match,
        ruleSet.ignore_resolution_difference,
        ruleSet.ignore_recompression,
        ruleSet.allow_rotation,
        ruleSet.allow_mirroring
    ];
    for (let index = 0; index < targetFlags.length; index += 1) {
        if (flags[index] !== targetFlags[index]) {
            return false;
        }
    }
    return true;
}

function checkThresholdsEquality(
    ruleSet: BackendDuplicateRuleSet,
    currentScores: number[]
): boolean {
    const targetScores = [ruleSet.min_score, ruleSet.min_visual_score, ruleSet.min_crop_score];
    for (let index = 0; index < targetScores.length; index += 1) {
        if (Math.abs(currentScores[index] - targetScores[index]) > 0.001) {
            return false;
        }
    }
    return true;
}

export function checkRuleSetEquality(
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
    const currentFlags = [
        exactMatch,
        visualMatch,
        cropMatch,
        ignoreResolution,
        ignoreCompression,
        allowRotation,
        allowMirroring
    ];
    if (!checkRuleFlagsEquality(ruleSet, currentFlags)) {
        return false;
    }
    const currentScores = [similarityScore, visualScore, cropScore];
    return checkThresholdsEquality(ruleSet, currentScores);
}
