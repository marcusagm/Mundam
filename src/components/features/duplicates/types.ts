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
