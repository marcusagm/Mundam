export interface DuplicateCandidate {
    id: string;
    name: string;
    size: string;
    sizeBytes: number;
    dimensions: string;
    score: number;
    path: string;
    format: string;
    createdAt: string;
    updatedAt: string;
    tags: { id: string; name: string }[];
    notes?: string;
    rating?: number;
    isFavorite: boolean;
    thumbnailUrl?: string;
    mediaType?: string;
    state?: string;
    isTrashed?: boolean;
    reasons?: string[];
}

export type DuplicateGroupStatus = 'open' | 'ignored' | 'resolved';

export interface DuplicateGroup {
    id: string;
    type: 'exact' | 'visual' | 'derived';
    status: DuplicateGroupStatus;
    confidence: number;
    candidateCount: number;
    candidates: DuplicateCandidate[];
    candidatesLoaded?: boolean;
}

/**
 * Predefined profiles for duplicate detection rules.
 */
export type DuplicateRuleProfile = 'exact-match' | 'visual-match' | 'aggressive' | 'custom';

/**
 * Rule configuration model for duplicate detection algorithm tuning.
 */
export interface DuplicateRuleConfiguration {
    /** Unique rule set identifier */
    id: string;
    /** Human-readable profile name */
    name: string;
    /** Optional rule set description */
    description?: string;
    /** Whether block-level exact content hashing is evaluated */
    considerExactMatch: boolean;
    /** Whether perceptual dHash and multiscale texture hashing are evaluated */
    considerVisualMatch: boolean;
    /** Whether spatial block hash sub-grid crop matching is evaluated */
    considerCropMatch: boolean;
    /** Whether resolution variations are ignored */
    ignoreResolutionDifference: boolean;
    /** Whether recompression between formats is ignored */
    ignoreRecompression: boolean;
    /** Minimum confidence score threshold (0.50 to 1.00) */
    minimumScore: number;
}
