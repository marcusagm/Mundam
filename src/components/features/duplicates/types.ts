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
}

export interface DuplicateGroup {
    id: string;
    type: 'exact' | 'visual' | 'derived';
    status: 'open' | 'ignored' | 'resolved';
    confidence: number;
    candidateCount: number;
    candidates: DuplicateCandidate[];
    candidatesLoaded?: boolean;
}
