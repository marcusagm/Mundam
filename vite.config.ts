import { defineConfig } from 'vite';
import solid from 'vite-plugin-solid';

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
    plugins: [solid()],

    // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
    //
    // 1. prevent Vite from obscuring rust errors
    clearScreen: false,
    // 2. tauri expects a fixed port, fail if that port is not available
    server: {
        port: 1420,
        strictPort: true,
        host: host || false,
        hmr: host
            ? {
                  protocol: 'ws',
                  host,
                  port: 1421
              }
            : undefined,
        watch: {
            // 3. tell Vite to ignore watching `src-tauri`
            ignored: ['**/src-tauri/**']
        }
    },
    build: {
        chunkSizeWarningLimit: 1000,
        rollupOptions: {
            input: {
                main: 'index.html',
                splashscreen: 'splashscreen.html'
            },
            output: {
                manualChunks(id) {
                    if (id.includes('@google/model-viewer')) {
                        return 'vendor-model-viewer';
                    }
                    if (id.includes('hls.js')) {
                        return 'vendor-hls';
                    }
                    if (id.includes('lucide-solid')) {
                        return 'vendor-icons';
                    }
                    if (id.includes('@tauri-apps')) {
                        return 'vendor-tauri';
                    }
                }
            }
        }
    }
}));
