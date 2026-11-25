import { GlobalStatusbar } from '../components/layout/GlobalStatusbar';
import { Component, JSX, createMemo } from 'solid-js';
import { open } from '@tauri-apps/plugin-dialog';
import { useSystem, useNotification } from '../core/hooks';
import { appearance } from '../core/store/appearanceStore';
import logoColor from '../assets/logo-color.svg';
import logoWhite from '../assets/logo-white.svg';

export interface WelcomeViewProperties {
    /** The title bar to render at the top */
    header: JSX.Element;
}

/**
 * The Welcome screen shown when no library folder is configured.
 * Prompts the user to select a root folder to initialize the application.
 *
 * @param {WelcomeViewProperties} props - Component properties.
 * @returns {JSX.Element} The rendered welcome screen.
 */
export const WelcomeView: Component<WelcomeViewProperties> = props => {
    const system = useSystem();
    const notification = useNotification();

    const effectiveLogo = createMemo(() => {
        let mode = appearance().mode;
        if (mode === 'system') {
            mode = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
        }
        return mode === 'dark' ? logoWhite : logoColor;
    });

    /**
     * Handles the folder selection dialog and library initialization.
     */
    const handleSelectFolder = async () => {
        try {
            const selected = await open({
                directory: true,
                multiple: false,
                title: 'Select Reference Library Folder'
            });

            if (selected) {
                const path = typeof selected === 'string' ? selected : String(selected);
                if (path) {
                    notification.info(
                        'Indexing Started',
                        `Processing folder: ${path.split(/[\\/]/).pop()}`
                    );
                    await system.setRootLocation(path);
                }
            }
        } catch (err) {
            console.error('Failed to select folder:', err);
        }
    };

    return (
        <div class="welcome-screen-container">
            {props.header}
            <div class="welcome-screen">
                <img src={effectiveLogo()} alt="Mundam Logo" class="welcome-logo" />
                <p>Start by choosing a folder to monitor for visual references.</p>
                <button class="primary-btn" onClick={handleSelectFolder}>
                    Initialize Library
                </button>
            </div>
            <GlobalStatusbar />
        </div>
    );
};
