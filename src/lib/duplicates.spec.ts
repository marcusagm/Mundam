import { describe, it, expect, vi, beforeEach } from 'vitest';
import { duplicatesApi } from './duplicates';
import { invokeCommand } from './api';

vi.mock('./api', () => ({
    invokeCommand: vi.fn()
}));

describe('duplicatesApi batch candidate loading', () => {
    const mockedInvokeCommand = vi.mocked(invokeCommand);

    beforeEach(() => {
        vi.clearAllMocks();
    });

    it('should return an empty list when group has no candidates', async () => {
        mockedInvokeCommand.mockResolvedValueOnce([]);

        const resolvedCandidates =
            await duplicatesApi.getDuplicateCandidates('empty-group-identifier');

        expect(resolvedCandidates).toEqual([]);
        expect(mockedInvokeCommand).toHaveBeenCalledTimes(1);
        expect(mockedInvokeCommand).toHaveBeenCalledWith('get_duplicate_candidates', {
            groupId: 'empty-group-identifier'
        });
    });

    it('should fetch assets and tags in batch without iterative N+1 calls', async () => {
        const mockBackendCandidates = [
            {
                group_id: 'group-1',
                asset_id: 'asset-identifier-1',
                score: 1.0,
                reasons: JSON.stringify(['Exact match']),
                is_selected: false
            },
            {
                group_id: 'group-1',
                asset_id: 'asset-identifier-2',
                score: 0.95,
                reasons: JSON.stringify(['Visual similarity']),
                is_selected: false
            }
        ];

        const mockBackendAssets = [
            {
                id: 'asset-identifier-1',
                path: '/photos/nature.jpg',
                state: 'Idle',
                size: 1048576,
                width: 1920,
                height: 1080,
                format: 'image/jpeg',
                media_type: 'IMAGE',
                created_at: '2026-01-01T00:00:00Z',
                updated_at: '2026-01-02T00:00:00Z',
                is_favorite: true
            },
            {
                id: 'asset-identifier-2',
                path: '/photos/nature_copy.jpg',
                state: 'Idle',
                size: 1048576,
                width: 1920,
                height: 1080,
                format: 'image/jpeg',
                media_type: 'IMAGE',
                created_at: '2026-01-01T00:00:00Z',
                updated_at: '2026-01-02T00:00:00Z',
                is_favorite: false
            }
        ];

        const mockTagsByAssetIdentifier = {
            'asset-identifier-1': [
                {
                    id: 'tag-1',
                    name: 'Landscape',
                    parent_id: null,
                    color: '#00ff00',
                    order_index: 0
                }
            ],
            'asset-identifier-2': []
        };

        mockedInvokeCommand.mockImplementation(async (commandName, payload) => {
            if (commandName === 'get_duplicate_candidates') {
                return mockBackendCandidates;
            }
            if (commandName === 'get_assets_by_ids') {
                return mockBackendAssets;
            }
            if (commandName === 'get_tags_for_assets') {
                return mockTagsByAssetIdentifier;
            }
            throw new Error(
                `Unexpected command called: ${commandName} with payload: ${JSON.stringify(payload)}`
            );
        });

        const resolvedCandidates = await duplicatesApi.getDuplicateCandidates('group-1');

        expect(resolvedCandidates).toHaveLength(2);
        expect(resolvedCandidates[0].id).toBe('asset-identifier-1');
        expect(resolvedCandidates[0].name).toBe('nature.jpg');
        expect(resolvedCandidates[0].tags).toEqual([{ id: 'tag-1', name: 'Landscape' }]);
        expect(resolvedCandidates[0].reasons).toEqual(['Exact match']);

        expect(resolvedCandidates[1].id).toBe('asset-identifier-2');
        expect(resolvedCandidates[1].name).toBe('nature_copy.jpg');
        expect(resolvedCandidates[1].tags).toEqual([]);
        expect(resolvedCandidates[1].reasons).toEqual(['Visual similarity']);

        // Must NOT call get_asset or get_tags_for_asset in an N+1 loop
        const commandNamesCalled = mockedInvokeCommand.mock.calls.map(call => call[0]);
        expect(commandNamesCalled).not.toContain('get_asset');
        expect(commandNamesCalled).not.toContain('get_tags_for_asset');

        // Must call get_duplicate_candidates, get_assets_by_ids, and get_tags_for_assets exactly once each
        expect(commandNamesCalled).toEqual([
            'get_duplicate_candidates',
            'get_assets_by_ids',
            'get_tags_for_assets'
        ]);

        expect(mockedInvokeCommand).toHaveBeenCalledWith('get_assets_by_ids', {
            assetIds: ['asset-identifier-1', 'asset-identifier-2']
        });
        expect(mockedInvokeCommand).toHaveBeenCalledWith('get_tags_for_assets', {
            assetIds: ['asset-identifier-1', 'asset-identifier-2']
        });
    });

    it('should invoke undo_duplicate_resolution command with group identifier', async () => {
        mockedInvokeCommand.mockResolvedValueOnce(undefined);

        await duplicatesApi.undoDuplicateResolution('target-duplicate-group-identifier');

        expect(mockedInvokeCommand).toHaveBeenCalledTimes(1);
        expect(mockedInvokeCommand).toHaveBeenCalledWith('undo_duplicate_resolution', {
            groupId: 'target-duplicate-group-identifier'
        });
    });
});
