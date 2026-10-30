import { Show } from 'solid-js';
import { useStartupProgress } from '../../../core/startup/useStartupProgress';
import logoWhite from '../../../assets/logo-white.svg';
import logoBlack from '../../../assets/logo-black.svg';
import './splash-screen.css';

/**
 * Lightweight startup presentation with real milestones and retained failures.
 * @returns The standalone, keyboard-accessible splash content.
 * @example
 * render(() => <SplashScreen />, document.getElementById('root')!);
 */
export function SplashScreen() {
    const startup = useStartupProgress();

    return (
        <main class="startup-screen" aria-label="Mundam startup">
            <button
                class="startup-screen__close"
                aria-label="Quit Mundam"
                onClick={() => void startup.close()}
            >
                <span aria-hidden="true">×</span>
            </button>
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
                <p class="startup-screen__tagline">A home for your inspiration.</p>
            </div>
            <div class="startup-screen__status">
                <Show when={!startup.failed()}>
                    <div
                        class="startup-screen__track"
                        role="progressbar"
                        aria-label="Starting Mundam"
                        aria-valuetext={startup.message()}
                    >
                        <span
                            classList={{
                                'startup-screen__indicator': true,
                                'startup-screen__indicator--paused': startup.unavailable()
                            }}
                        />
                    </div>
                </Show>
                <p class="startup-screen__message" role="status" aria-live="polite">
                    {startup.message()}
                </p>
                <Show
                    when={startup.failed()}
                    fallback={
                        <Show when={startup.isSlow()}>
                            <p class="startup-screen__detail">
                                Taking longer than usual · {startup.elapsedSeconds()}s
                            </p>
                        </Show>
                    }
                >
                    <p class="startup-screen__detail">Please quit and reopen Mundam.</p>
                    <code class="startup-screen__diagnostic">{startup.snapshot()?.errorCode}</code>
                    <button class="startup-screen__quit" onClick={() => void startup.close()}>
                        Quit Mundam
                    </button>
                </Show>
                <Show when={startup.closeError()}>
                    <p role="alert" class="startup-screen__detail">
                        Unable to quit. Close Mundam using your system controls.
                    </p>
                </Show>
            </div>
        </main>
    );
}
