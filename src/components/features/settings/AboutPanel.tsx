import { Component, For, createMemo } from 'solid-js';
import { openUrl } from '@tauri-apps/plugin-opener';
import { appearance } from '../../../core/store/appearanceStore';
import logoColor from '../../../assets/logo-color.svg';
import logoWhite from '../../../assets/logo-white.svg';
import './about-panel.css';

interface TechItem {
    name: string;
    description: string;
    url: string;
}

interface TechCategory {
    title: string;
    items: TechItem[];
}

const TECH_STACK: TechCategory[] = [
    {
        title: 'Core Infrastructure',
        items: [
            {
                name: 'Tauri v2',
                description: 'Secure desktop framework building bridges between Rust and JS.',
                url: 'https://tauri.app/'
            },
            {
                name: 'SolidJS',
                description: 'Performant, reactive UI framework with zero-overhead.',
                url: 'https://www.solidjs.com/'
            },
            {
                name: 'Rust',
                description: 'The safe, fast engine powering all native operations.',
                url: 'https://www.rust-lang.org/'
            },
            {
                name: 'Vite',
                description: 'Next-generation frontend tooling and build system.',
                url: 'https://vitejs.dev/'
            }
        ]
    },
    {
        title: 'Native Engine (Rust & C++)',
        items: [
            {
                name: 'FFmpeg',
                description: 'High-performance multimedia processing and RAW decoding.',
                url: 'https://ffmpeg.org/'
            },
            {
                name: 'Assimp',
                description: 'Open Asset Import Library for 3D model standardization.',
                url: 'https://www.assimp.org/'
            },
            {
                name: 'Zune-JPEG & WebP',
                description: 'Elite level image codecs for near-instant rendering.',
                url: 'https://github.com/etemesi254/zune-image'
            },
            {
                name: 'Fast Image Resize',
                description: 'Simd-accelerated thumbnail generation performance.',
                url: 'https://github.com/Cykooz/fast_image_resize'
            },
            {
                name: 'Resvg & Tiny-skia',
                description: 'Static-analysis based SVG rendering engine.',
                url: 'https://github.com/linebender/resvg'
            },
            {
                name: 'SQLx & SQLite',
                description: 'Reliable, local-first database management system.',
                url: 'https://github.com/launchbadge/sqlx'
            },
            {
                name: 'Tokio & Rayon',
                description: 'Advanced async and parallel task scheduling.',
                url: 'https://tokio.rs/'
            },
            {
                name: 'Wuff',
                description: 'Secure WOFF/WOFF2 font file decoding.',
                url: 'https://github.com/google/wuffs'
            }
        ]
    },
    {
        title: 'Frontend & UI',
        items: [
            {
                name: 'Model Viewer',
                description: "Interactive 3D viewing via Google's web components.",
                url: 'https://modelviewer.dev/'
            },
            {
                name: 'Lucide Solid',
                description: 'Beautifully crafted, consistent icon set.',
                url: 'https://lucide.dev/'
            },
            {
                name: 'Solid DnD',
                description: 'Accessible drag and drop primitives for SolidJS.',
                url: 'https://github.com/thisbeyond/solid-dnd'
            },
            {
                name: 'Floating UI',
                description: 'Low-level library for positioning tooltips and popovers.',
                url: 'https://floating-ui.com/'
            }
        ]
    }
];

export const AboutPanel: Component = () => {
    const handleOpenLink = (url: string) => {
        openUrl(url).catch((err: unknown) => console.error('Failed to open link:', err));
    };

    const effectiveLogo = createMemo(() => {
        let mode = appearance().mode;
        if (mode === 'system') {
            mode = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
        }
        return mode === 'dark' ? logoWhite : logoColor;
    });

    return (
        <div class="settings-panel-content about-panel">
            {/* Hero Section */}
            <div class="about-hero">
                <div class="about-logo-container">
                    <img
                        id="about-app-logo"
                        src={effectiveLogo()}
                        alt="Mundam Logo"
                        class="about-logo"
                    />
                </div>
                {/* <h1 class="about-app-name">Mundam</h1> */}
                <div class="about-version">Version 0.1.0</div>
            </div>

            {/* Tech Stack */}
            <div class="about-sections">
                <For each={TECH_STACK}>
                    {category => (
                        <div class="about-category">
                            <h3 class="about-section-title">{category.title}</h3>
                            <div class="tech-grid">
                                <For each={category.items}>
                                    {tech => (
                                        <button
                                            class="tech-card"
                                            onClick={() => handleOpenLink(tech.url)}
                                            title={`Open ${tech.name} website`}
                                        >
                                            <span class="tech-name">{tech.name}</span>
                                            <span class="tech-desc">{tech.description}</span>
                                        </button>
                                    )}
                                </For>
                            </div>
                        </div>
                    )}
                </For>
            </div>

            {/* Footer */}
            <footer class="about-footer">
                <p class="about-copyright">© 2024-2025 Marcus Maia. All rights reserved.</p>
                <div class="about-links">
                    <button
                        onClick={() => handleOpenLink('https://github.com/marcusagm/Mundam')}
                        class="about-link"
                    >
                        GitHub
                    </button>
                    <button onClick={() => handleOpenLink('https://mundam.app')} class="about-link">
                        Website
                    </button>
                    <button
                        onClick={() => handleOpenLink('https://github.com/marcusagm/Mundam/issues')}
                        class="about-link"
                    >
                        Support
                    </button>
                </div>
            </footer>
        </div>
    );
};
