import { dirname, join } from 'node:path';
import { loadProfile } from './lib/import-client-storage.mjs';
import { windowsDiscoveryPath } from './lib/import-discovery.mjs';
import { createSession } from './lib/import-session.mjs';
import { runNativeBridge } from './lib/native-import-bridge.mjs';

let session;
async function send(...request) {
    if (!session) {
        const discovery = windowsDiscoveryPath();
        const profile = await loadProfile(join(dirname(discovery), 'import-browser.v1.dpapi'));
        session = createSession(profile, discovery);
    }
    return session(...request);
}

try {
    await runNativeBridge(process.stdin, process.stdout, send);
} catch {
    console.error('Mundam native bridge closed because its stream could not be read or written.');
    process.exitCode = 1;
}
