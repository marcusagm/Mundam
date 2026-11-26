import { createHash, X509Certificate } from 'node:crypto';
import { once } from 'node:events';
import https from 'node:https';
import { checkServerIdentity } from 'node:tls';

/** Computes the SHA-256 used by receipts and the TLS public-key pin. */
export const digest = bytes => createHash('sha256').update(bytes).digest('hex');

/** Computes the pin of an X.509 certificate's SubjectPublicKeyInfo. */
export function publicKeyPin(certificate) {
    return digest(certificate.publicKey.export({ format: 'der', type: 'spki' }));
}

/** Opens a pinned TLS connection to the fixed IPv4 loopback address. */
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
    request.setTimeout(10_000, () => {
        request.destroy(
            Object.assign(new Error('Mundam request timed out'), { code: 'ETIMEDOUT' })
        );
    });
    return request;
}

/** Sends a request over the pinned connection and reads one bounded JSON response. */
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
    let size = 0;
    for await (const chunk of incoming) {
        size += chunk.length;
        if (size > 256 * 1024) throw new Error('Mundam response exceeds the size limit.');
        chunks.push(chunk);
    }
    const text = Buffer.concat(chunks).toString('utf8');
    return { status: incoming.statusCode, data: text ? JSON.parse(text) : null };
}
