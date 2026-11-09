import { TechCategory } from './types';

export const frontendDependencies: TechCategory = {
    title: 'Frontend Dependencies',
    items: [
        {
            name: '@floating-ui/dom',
            description: 'Positioning tooltips and popovers.',
            url: 'https://floating-ui.com/'
        },
        {
            name: '@google/model-viewer',
            description: 'Interactive 3D viewing web component.',
            url: 'https://modelviewer.dev/'
        },
        {
            name: '@hypothesi/tauri-plugin-mcp-bridge',
            description: 'Connects Tauri to the MCP bridge integration.',
            url: 'https://www.npmjs.com/package/@hypothesi/tauri-plugin-mcp-bridge'
        },
        {
            name: '@thisbeyond/solid-dnd',
            description: 'Accessible drag and drop for SolidJS.',
            url: 'https://github.com/thisbeyond/solid-dnd'
        },
        {
            name: 'fuse.js',
            description: 'Lightweight fuzzy-search logic.',
            url: 'https://fusejs.io/'
        },
        {
            name: 'hls.js',
            description: 'JavaScript HLS video player.',
            url: 'https://github.com/video-dev/hls.js'
        },
        {
            name: 'lucide-solid',
            description: 'Beautifully crafted, consistent icon set.',
            url: 'https://lucide.dev/'
        },
        {
            name: 'zod & zod-validation-error',
            description: 'TypeScript-first schema validation.',
            url: 'https://zod.dev/'
        }
    ]
};
