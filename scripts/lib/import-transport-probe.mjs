import { execFileSync, spawn } from 'node:child_process';
import { createPrivateKey, randomBytes, X509Certificate } from 'node:crypto';
import { once } from 'node:events';
import { readFile, unlink } from 'node:fs/promises';
import { join } from 'node:path';
import { createInterface } from 'node:readline';
import { publicKeyPin } from './import-http.mjs';
export { digest, exchange, openRequest } from './import-http.mjs';
import { fileURLToPath } from 'node:url';

export function buildProbe(example = 'import_transport') {
    const output = execFileSync(
        'cargo',
        ['build', '--locked', '--example', example, '--message-format=json-render-diagnostics'],
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
        if (artifact.target?.name === example && artifact.executable) return artifact.executable;
    }
    throw new Error('Cargo did not produce the import transport probe');
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

export async function startServer(executable, database, identity, controlled = false) {
    const child = spawn(executable, [database], {
        stdio: ['pipe', 'pipe', 'inherit'],
        windowsHide: true
    });
    const closed = once(child, 'close');
    const lines = createInterface({ input: child.stdout });
    const ended = closed.then(([code, signal]) => {
        throw new Error(`Probe exited before responding (code ${code}, signal ${signal})`);
    });
    void ended.catch(() => {});
    let timer;
    try {
        const ready = once(lines, 'line');
        const inputFailure = new Promise((_, reject) => child.stdin.once('error', reject));
        const configuration = JSON.stringify(identity.config);
        if (controlled) child.stdin.write(configuration + '\n');
        else child.stdin.end(configuration);
        const [line] = await Promise.race([
            ready,
            inputFailure,
            ended,
            new Promise((_, reject) => {
                timer = setTimeout(() => reject(new Error('Probe startup timed out')), 20_000);
            })
        ]);
        const readyData = JSON.parse(line);
        const { port } = readyData;
        if (!Number.isInteger(port) || port < 1 || port > 65535)
            throw new Error('Invalid probe endpoint');
        return {
            port,
            readyData,
            async desktop(action) {
                const output = once(lines, 'line');
                child.stdin.write(JSON.stringify(action) + '\n');
                const [line] = await Promise.race([output, ended]);
                return JSON.parse(line);
            },
            async shutdown() {
                child.stdin.end();
                const [code] = await closed;
                if (code !== 0) throw new Error(`Probe shutdown failed (code ${code})`);
            },
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
        if (!controlled) lines.close();
    }
}
