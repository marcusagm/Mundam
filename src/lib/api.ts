import { invoke as tauriInvoke, InvokeArgs } from '@tauri-apps/api/core';
import { toast } from '../components/ui/Sonner/state';

const ERROR_MAP: Record<string, { variant: 'error' | 'warning'; title: string }> = {
    NOT_FOUND: { variant: 'error', title: 'Não Encontrado' },
    DATABASE_ERROR: { variant: 'error', title: 'Erro de Banco de Dados' },
    VALIDATION_FAILED: { variant: 'warning', title: 'Validação Falhou' },
    FORMAT_NOT_SUPPORTED: { variant: 'error', title: 'Formato Inválido' },
    IO_ERROR: { variant: 'error', title: 'Erro de Arquivo' },
    TIMEOUT: { variant: 'warning', title: 'Tempo Esgotado' }
};

/**
 * Internal helper to translate structured backend payload errors
 * into User-facing Toasts, avoiding async function complexity limits.
 */
function handleBackendError(error: unknown): void {
    if (!error || typeof error !== 'object' || !('code' in error)) {
        toast.error('Erro de Sistema', { description: String(error) });
        return;
    }

    const err = error as Record<string, unknown>;
    const message = typeof err.message === 'string' ? err.message : 'Erro desconhecido.';
    const codeKey = String(err.code);
    const mapping = ERROR_MAP[codeKey] || { variant: 'error', title: 'Falha no Sistema' };

    if (mapping.variant === 'warning') {
        toast.warning(mapping.title, { description: message });
    } else {
        toast.error(mapping.title, { description: message });
    }
}

/**
 * Universal Wrapper for Tauri RPC Calls.
 *
 * Enforces the Hexagonal Architecture rule where errors coming from the Backend
 * are structured JSON payloads matching the `AppError` enum mapping.
 * This wrapper catches those errors globally and fires the corresponding Toasts
 * so that Views don't have to duplicate error handling logic.
 */
export async function invokeCommand<T>(command: string, args?: InvokeArgs): Promise<T> {
    try {
        return await tauriInvoke<T>(command, args);
    } catch (error: unknown) {
        if (error && typeof error === 'object' && 'code' in error && error.code === 'CANCELLED') {
            throw error;
        }
        console.error(`[IPC Error: ${command}]`, error);
        handleBackendError(error);
        throw error;
    }
}
