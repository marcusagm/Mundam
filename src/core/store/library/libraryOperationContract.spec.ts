import { expect, it } from 'vitest';
import fixture from './fixtures/libraryOperationResult.json';
import type { LibraryOperationResult } from './folderActions';

it('keeps the TypeScript receipt synchronized with the shared Rust serialization fixture', () => {
    const result: LibraryOperationResult = {
        operationId: fixture.operationId,
        id: fixture.id,
        entityType: fixture.entityType === 'folder' ? 'folder' : 'asset',
        path: fixture.path,
        previousPath: fixture.previousPath,
        effects: fixture.effects,
        warning: fixture.warning
    };
    expect(result).toEqual(fixture);
});
