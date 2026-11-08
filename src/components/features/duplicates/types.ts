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
    /** Whether to check rotated (90°/180°/270°) hash variants */
    allowRotation: boolean;
    /** Whether to check horizontally mirrored hash variants */
    allowMirroring: boolean;
    /** Minimum confidence score threshold (0.50 to 1.00) */
    minimumScore: number;
    /** Independent minimum score for visual/perceptual matches (0.50 to 1.00) */
    minimumVisualScore: number;
    /** Independent minimum score for crop/derived matches (0.50 to 1.00) */
    minimumCropScore: number;
}

/**
 * Properties for the DuplicateRulesForm component.
 */
export interface DuplicateRulesFormProperties {
    /** Currently selected profile identifier ('exact-match', 'visual-match', 'aggressive', 'custom') */
    selectedProfile: DuplicateRuleProfile;
    /** Whether block-level SHA-256 exact matching is active */
    considerExactMatch: boolean;
    /** Whether perceptual gradient visual matching is active */
    considerVisualMatch: boolean;
    /** Whether 4x4 spatial block hash crop matching is active */
    considerCropMatch: boolean;
    /** Whether resolution differences are ignored */
    ignoreResolutionDifference: boolean;
    /** Whether format recompressions are ignored */
    ignoreRecompression: boolean;
    /** Whether rotated (90°/180°/270°) hash variants are tested */
    allowRotation: boolean;
    /** Whether horizontally mirrored hash variants are tested */
    allowMirroring: boolean;
    /** Current shared similarity threshold (0.50 to 1.00) */
    minimumScore: number;
    /** Current visual match threshold (0.50 to 1.00) */
    minimumVisualScore: number;
    /** Current crop match threshold (0.50 to 1.00) */
    minimumCropScore: number;
    /** Whether controls are disabled */
    isDisabled?: boolean;
    /** Callback when profile changes */
    onProfileChange: (profileIdentifier: string) => void;
    /** Callback when exact match toggle changes */
    onConsiderExactMatchChange: (checked: boolean) => void;
    /** Callback when visual match toggle changes */
    onConsiderVisualMatchChange: (checked: boolean) => void;
    /** Callback when crop match toggle changes */
    onConsiderCropMatchChange: (checked: boolean) => void;
    /** Callback when ignore resolution toggle changes */
    onIgnoreResolutionDifferenceChange: (checked: boolean) => void;
    /** Callback when ignore recompression toggle changes */
    onIgnoreRecompressionChange: (checked: boolean) => void;
    /** Callback when rotation toggle changes */
    onAllowRotationChange: (checked: boolean) => void;
    /** Callback when mirroring toggle changes */
    onAllowMirroringChange: (checked: boolean) => void;
    /** Callback when minimum score slider changes */
    onMinimumScoreChange: (value: number) => void;
    /** Callback when visual score slider changes */
    onMinimumVisualScoreChange: (value: number) => void;
    /** Callback when crop score slider changes */
    onMinimumCropScoreChange: (value: number) => void;
}
