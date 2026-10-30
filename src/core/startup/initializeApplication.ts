import { invoke, isTauri } from '@tauri-apps/api/core';
import { waitForBackend } from './client';

/**
 * Gates application imports and rendering on essential backend/frontend state.
 * Failures remain visible in the independent splash.
 * @returns Resolves once the main window is revealed or failure is reported.
 * @example
 * void initializeApplication();
 */
export async function initializeApplication(): Promise<void> {
    const desktop = isTauri();
    try {
        if (desktop) await waitForBackend();
        const { mountApplication } = await import('../../mountApplication');
        await mountApplication(desktop);
        if (desktop && window.location.hash !== '#design-system') await invoke('complete_startup');
    } catch (error) {
        console.error('Application startup failed:', error);
        if (desktop && window.location.hash !== '#design-system') {
            await invoke('fail_frontend_startup').catch(reportError => {
                console.error('Unable to report startup failure:', reportError);
            });
        } else {
            const root = document.getElementById('root');
            if (root) root.textContent = 'Unable to start Mundam. Please reload to try again.';
        }
    }
}
