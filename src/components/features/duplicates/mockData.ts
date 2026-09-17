import { DuplicateGroup } from './types';

export const mockGroups: DuplicateGroup[] = [
    {
        id: 'group-1',
        type: 'exact',
        status: 'open',
        confidence: 1.0,
        candidateCount: 2,
        candidates: [
            {
                id: 'candidate-1',
                name: 'DSC001.jpg',
                size: '2.4 MB',
                sizeBytes: 2516582,
                dimensions: '4000x3000',
                score: 1.0,
                path: '/Volumes/Photos/2023/DSC001.jpg',
                format: 'JPEG',
                createdAt: '2023-05-12T10:30:00Z',
                updatedAt: '2023-05-12T10:30:00Z',
                tags: [
                    { id: 'tag-vacation', name: 'vacation' },
                    { id: 'tag-beach', name: 'beach' }
                ],
                notes: 'Original photo',
                isFavorite: true,
                mediaType: 'Image'
            },
            {
                id: 'candidate-2',
                name: 'DSC001_copy.jpg',
                size: '2.4 MB',
                sizeBytes: 2516582,
                dimensions: '4000x3000',
                score: 1.0,
                path: '/Users/marcus/Downloads/DSC001_copy.jpg',
                format: 'JPEG',
                createdAt: '2023-06-01T14:20:00Z',
                updatedAt: '2023-06-01T14:20:00Z',
                tags: [],
                isFavorite: false,
                mediaType: 'Image'
            }
        ]
    },
    {
        id: 'group-2',
        type: 'visual',
        status: 'open',
        confidence: 0.94,
        candidateCount: 2,
        candidates: [
            {
                id: 'candidate-3',
                name: 'Profile.png',
                size: '1.1 MB',
                sizeBytes: 1153434,
                dimensions: '1080x1080',
                score: 1.0,
                path: '/Users/marcus/Pictures/Profile.png',
                format: 'PNG',
                createdAt: '2024-01-15T09:00:00Z',
                updatedAt: '2024-01-15T09:00:00Z',
                tags: [
                    { id: 'tag-profile', name: 'profile' },
                    { id: 'tag-work', name: 'work' }
                ],
                notes: 'High res profile picture',
                isFavorite: true,
                mediaType: 'Image'
            },
            {
                id: 'candidate-4',
                name: 'Profile_web.jpg',
                size: '300 KB',
                sizeBytes: 307200,
                dimensions: '1080x1080',
                score: 0.94,
                path: '/Users/marcus/Projects/website/assets/Profile_web.jpg',
                format: 'JPEG',
                createdAt: '2024-01-15T09:15:00Z',
                updatedAt: '2024-01-15T09:15:00Z',
                tags: [{ id: 'tag-web', name: 'web' }],
                isFavorite: false,
                mediaType: 'Image'
            }
        ]
    }
];
