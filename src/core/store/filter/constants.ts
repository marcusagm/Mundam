export const SIZE_UNITS = [
    { value: '1', label: 'Bytes' },
    { value: '1024', label: 'KB' },
    { value: '1048576', label: 'MB' },
    { value: '1073741824', label: 'GB' }
];

export const SEARCH_FIELDS = [
    { value: 'tags', label: 'Tags', type: 'tags' },
    { value: 'color', label: 'Color', type: 'color' },
    { value: 'filename', label: 'Filename', type: 'text' },
    { value: 'format', label: 'Format', type: 'select' },
    { value: 'size', label: 'File size', type: 'size' },
    { value: 'width', label: 'Width', type: 'number' },
    { value: 'height', label: 'Height', type: 'number' },
    { value: 'added_at', label: 'Date added', type: 'date' },
    { value: 'created_at', label: 'Date creation', type: 'date' },
    { value: 'modified_at', label: 'Date modified', type: 'date' },
    { value: 'rating', label: 'Rating', type: 'rating' },
    { value: 'keywords', label: 'Keywords', type: 'text' },
    { value: 'notes', label: 'Notes', type: 'text' },
    { value: 'folder', label: 'Folder', type: 'folder' }
];

export const OPERATORS_FOR_TYPE: Record<string, { value: string; label: string }[]> = {
    text: [
        { value: 'contains', label: 'Contains' },
        { value: 'not_contains', label: 'Not Contains' },
        { value: 'equals', label: 'Equals' },
        { value: 'starts_with', label: 'Starts With' },
        { value: 'ends_with', label: 'Ends With' }
    ],
    number: [
        { value: 'gt', label: 'Greater than' },
        { value: 'lt', label: 'Less than' },
        { value: 'eq', label: 'Equals' },
        { value: 'between', label: 'Between' }
    ],
    size: [
        { value: 'gt', label: 'Greater than' },
        { value: 'lt', label: 'Less than' },
        { value: 'eq', label: 'Equals' },
        { value: 'between', label: 'Between' }
    ],
    date: [
        { value: 'before', label: 'Before' },
        { value: 'after', label: 'After' },
        { value: 'on', label: 'On' },
        { value: 'between', label: 'Between' }
    ],
    select: [
        { value: 'eq', label: 'Equals' },
        { value: 'neq', label: 'Not Equals' }
    ],
    tags: [
        { value: 'contains', label: 'Contains' },
        { value: 'not_contains', label: 'Not Contains' }
    ],
    folder: [
        { value: 'is', label: 'Is' },
        { value: 'in', label: 'Is inside (recursive)' }
    ],
    rating: [
        { value: 'eq', label: 'Equals' },
        { value: 'gte', label: 'Greater than or equal' },
        { value: 'lte', label: 'Less than or equal' }
    ],
    color: [
        { value: 'similar', label: 'Similar to' },
        { value: 'exact', label: 'Exact match' }
    ]
};
