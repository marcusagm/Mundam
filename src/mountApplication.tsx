import { render } from 'solid-js/web';
import App from './App';
import { appearanceActions } from './core/store/appearanceStore';
import { formatActions } from './core/store/formatStore';
import { systemActions } from './core/store/systemStore';
import { reportStartupStage } from './core/startup/client';
import { InputProvider } from './core/input';
import './styles/global.css';

/**
 * Initializes stores before mounting the first usable application view.
 * @param desktop - Enables native startup reporting.
 * @returns Resolves after the application tree and fonts are prepared.
 * @throws Propagates essential initialization failures to the supervisor.
 * @example
 * await mountApplication(true);
 */
export async function mountApplication(desktop: boolean): Promise<void> {
    const root = document.getElementById('root');
    if (!root) throw new Error('Application root is missing');
    if (window.location.hash === '#design-system') {
        const { DesignSystemGuide } = await import('./components/features/design');
        render(
            () => (
                <InputProvider>
                    <DesignSystemGuide />
                </InputProvider>
            ),
            root
        );
        return;
    }
    if (desktop) await reportStartupStage('appearance');
    await appearanceActions.initialize();
    if (desktop) await reportStartupStage('content');
    await formatActions.initialize();
    await systemActions.initialize();
    if (desktop) await reportStartupStage('interface');
    render(() => <App />, root);
    await document.fonts.ready;
}
