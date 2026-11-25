import { execFileSync, spawn } from 'node:child_process';
import { createHash, createPrivateKey, randomBytes, X509Certificate } from 'node:crypto';
import { once } from 'node:events';
import { readFile, unlink } from 'node:fs/promises';
import https from 'node:https';
import { join } from 'node:path';
import { createInterface } from 'node:readline';
import { checkServerIdentity } from 'node:tls';
import { fileURLToPath } from 'node:url';

export function buildProbe() {
    const output = execFileSync(
        'cargo',
        [
            'build',
            '--locked',
            '--example',
            'import_transport',
            '--message-format=json-render-diagnostics'
        ],
        {
            cwd: fileURLToPath(new URL('../../src-tauri/', import.meta.url)),
            env:
                process.platform === 'win32'
                    ? {
                          ...process.env,
                          RUSTUP_TOOLCHAIN:
                              process.env.RUSTUP_TOOLCHAIN || 'stable-x86_64-pc-windows-msvc'
                      }
                    : process.env,
            encoding: 'utf8',
            stdio: ['ignore', 'pipe', 'inherit'],
            windowsHide: true,
            maxBuffer: 16 * 1024 * 1024
        }
    );
    for (const line of output.trim().split(/\r?\n/)) {
        const artifact = JSON.parse(line);
        if (artifact.target?.name === 'import_transport' && artifact.executable)
            return artifact.executable;
    }
    throw new Error('Cargo did not produce the import transport probe');
}

export function digest(bytes) {
    return createHash('sha256').update(bytes).digest('hex');
}

function publicKeyPin(certificate) {
    return digest(certificate.publicKey.export({ format: 'der', type: 'spki' }));
}

export async function createIdentity(directory) {
    const keyPath = join(directory, 'test-key.pem');
    const certificate = execFileSync(
        'openssl',
        [
            'req',
            '-new',
            '-x509',
            '-newkey',
            'ec',
            '-pkeyopt',
            'ec_paramgen_curve:prime256v1',
            '-nodes',
            '-keyout',
            keyPath,
            '-days',
            '1',
            '-subj',
            '/CN=Mundam transport test',
            '-addext',
            'subjectAltName=IP:127.0.0.1',
            '-addext',
            'extendedKeyUsage=serverAuth'
        ],
        { windowsHide: true }
    );
    const privateKey = createPrivateKey(await readFile(keyPath));
    await unlink(keyPath);
    const x509 = new X509Certificate(certificate);
    return {
        certificate,
        pin: publicKeyPin(x509),
        config: {
            certificateDer: x509.raw.toString('base64'),
            privateKeyDer: privateKey.export({ format: 'der', type: 'pkcs8' }).toString('base64'),
            ownerToken: randomBytes(32).toString('hex'),
            otherToken: randomBytes(32).toString('hex')
        }
    };
}

export async function startServer(executable, database, identity) {
    const child = spawn(executable, [database], {
        stdio: ['pipe', 'pipe', 'inherit'],
        windowsHide: true
    });
    const closed = once(child, 'close');
    const lines = createInterface({ input: child.stdout });
    let timer;
    try {
        const ready = once(lines, 'line');
        const inputFailure = new Promise((_, reject) => child.stdin.once('error', reject));
        child.stdin.end(JSON.stringify(identity.config));
        const [line] = await Promise.race([
            ready,
            inputFailure,
            closed.then(([code, signal]) => {
                throw new Error(`Probe exited before listening (code ${code}, signal ${signal})`);
            }),
            new Promise((_, reject) => {
                timer = setTimeout(() => reject(new Error('Probe startup timed out')), 20_000);
            })
        ]);
        const { port } = JSON.parse(line);
        if (!Number.isInteger(port) || port < 1 || port > 65535)
            throw new Error('Invalid probe endpoint');
        return {
            port,
            async stop() {
                child.kill('SIGKILL');
                await closed;
            }
        };
    } catch (error) {
        child.kill('SIGKILL');
        await closed;
        throw error;
    } finally {
        clearTimeout(timer);
        lines.close();
    }
}

export function openRequest(port, identity, method, path, headers, pin = identity.pin) {
    const request = https.request({
        hostname: '127.0.0.1',
        port,
        path,
        method,
        headers,
        agent: false,
        ca: identity.certificate,
        rejectUnauthorized: true,
        minVersion: 'TLSv1.3',
        checkServerIdentity(host, peer) {
            return (
                checkServerIdentity(host, peer) ??
                (publicKeyPin(new X509Certificate(peer.raw)) === pin
                    ? undefined
                    : new Error('Server key pin mismatch'))
            );
        }
    });
    request.setTimeout(10_000, () => request.destroy(new Error('Probe request timed out')));
    return request;
}

export async function exchange(
    server,
    identity,
    method,
    path,
    { token, body, headers = {}, pin } = {}
) {
    const request = openRequest(
        server.port,
        identity,
        method,
        path,
        {
            ...(token ? { authorization: `Bearer ${token}` } : {}),
            ...headers
        },
        pin
    );
    const response = once(request, 'response');
    request.end(body);
    const [incoming] = await response;
    const chunks = [];
    for await (const chunk of incoming) chunks.push(chunk);
    const text = Buffer.concat(chunks).toString('utf8');
    return { status: incoming.statusCode, data: text ? JSON.parse(text) : null };
}
