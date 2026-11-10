import { Show } from 'solid-js';
import { useShutdownProgress } from '../../../core/shutdown/useShutdownProgress';
import logoWhite from '../../../assets/logo-white.svg';
import logoBlack from '../../../assets/logo-black.svg';
import '../startup/splash-screen.css';
import './shutdown-screen.css';

/**
 * Presents application shutdown without depending on the main interface.
 * @returns The standalone shutdown presentation.
 * @example
 * render(() => <ShutdownScreen />, document.getElementById('root')!);
 */
export function ShutdownScreen() {
    const shutdown = useShutdownProgress();
    return (
        <main class="startup-screen shutdown-screen" aria-labelledby="shutdown-title">
            <div class="startup-screen__brand">
                <img
                    class="startup-screen__logo startup-screen__logo--dark"
                    src={logoWhite}
                    alt="Mundam"
                />
                <img
                    class="startup-screen__logo startup-screen__logo--light"
                    src={logoBlack}
                    alt="Mundam"
                />
            </div>
            <div class="startup-screen__status">
                <h1 id="shutdown-title" class="shutdown-screen__title">
                    Closing Mundam
                </h1>
                <div
                    class="startup-screen__track"
                    role="progressbar"
                    aria-label="Closing Mundam"
                    aria-valuetext={shutdown.message()}
                >
                    <span
                        classList={{
                            'startup-screen__indicator': true,
                            'startup-screen__indicator--paused': shutdown.unavailable()
                        }}
                    />
                </div>
                <p class="startup-screen__message" role="status" aria-live="polite">
                    {shutdown.message()}
                </p>
                <Show when={shutdown.isSlow()}>
                    <p class="startup-screen__detail">
                        Taking longer than usual · {shutdown.elapsedSeconds()}s
                    </p>
                </Show>
            </div>
        </main>
    );
}
