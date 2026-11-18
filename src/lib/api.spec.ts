import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invokeCommand } from './api';

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), error: vi.fn(), warning: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }));
vi.mock('../components/ui/Sonner/state', () => ({
    toast: { error: mocks.error, warning: mocks.warning }
}));

describe('IPC shutdown cancellation', () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    it('propagates cancellation without displaying a system failure', async () => {
        const cancelled = { code: 'CANCELLED', message: 'Operation cancelled.' };
        mocks.invoke.mockRejectedValueOnce(cancelled);
        await expect(invokeCommand('get_assets')).rejects.toBe(cancelled);
        expect(mocks.error).not.toHaveBeenCalled();
        expect(mocks.warning).not.toHaveBeenCalled();
    });

    it('still displays real database failures', async () => {
        const failure = { code: 'DATABASE_ERROR', message: 'Database failed' };
        mocks.invoke.mockRejectedValueOnce(failure);
        await expect(invokeCommand('get_assets')).rejects.toBe(failure);
        expect(mocks.error).toHaveBeenCalledWith('Erro de Banco de Dados', {
            description: 'Database failed'
        });
    });

    it('returns successful results unchanged', async () => {
        mocks.invoke.mockResolvedValueOnce({ items: [] });
        await expect(invokeCommand('get_assets')).resolves.toEqual({ items: [] });
        expect(mocks.error).not.toHaveBeenCalled();
    });
});
