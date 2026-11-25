import { open } from 'node:fs/promises';
import { join } from 'node:path';

/** Returns the Windows discovery file path for the current user. */
export function windowsDiscoveryPath() {
    if (process.platform !== 'win32' || !process.env.LOCALAPPDATA) {
        throw new Error('Automatic discovery is currently available on Windows.');
    }
    return join(process.env.LOCALAPPDATA, 'com.marcusmaia.mundam', 'import-endpoint.json');
}

/** Parses an explicit local HTTPS endpoint without allowing another host or credentials. */
export function loopbackPort(url) {
    if (typeof url !== 'string') return null;
    const match = /^https:\/\/127\.0\.0\.1:([1-9][0-9]{0,4})\/?$/.exec(url);
    if (!match || match[0] !== url) return null;
    const port = Number(match[1]);
    return port <= 65535 ? port : null;
}

/** Returns a connection hint for a previously trusted pin. Verify TLS using the
 * original invitation certificate and pin before sending a bearer.
 */
export async function readDiscovery(path, trustedPin) {
    let file;
    try {
        file = await open(path, 'r');
    } catch (error) {
        if (error.code === 'ENOENT') return null;
        throw error;
    }
    try {
        if (!(await file.stat()).isFile()) return null;
        const buffer = Buffer.alloc(4097);
        let length = 0;
        while (length < buffer.length) {
            const { bytesRead } = await file.read(buffer, length, buffer.length - length, length);
            if (!bytesRead) break;
            length += bytesRead;
        }
        if (length === buffer.length) return null;
        let document;
        try {
            document = JSON.parse(buffer.toString('utf8', 0, length));
        } catch {
            return null;
        }
        const port = loopbackPort(document?.url);
        if (
            document?.version !== 1 ||
            !port ||
            !/^[a-f0-9]{64}$/.test(trustedPin) ||
            document.publicKeySha256 !== trustedPin ||
            !/^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/.test(
                document.instanceId
            )
        ) {
            return null;
        }
        return { port, instanceId: document.instanceId };
    } finally {
        await file.close();
    }
}
