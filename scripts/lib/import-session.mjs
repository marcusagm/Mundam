import { X509Certificate } from 'node:crypto';
import { setTimeout as pause } from 'node:timers/promises';
import { exchange, publicKeyPin } from './import-http.mjs';
import { loopbackPort, readDiscovery } from './import-discovery.mjs';

/** Parses a bounded invitation supplied through the trusted desktop channel. */
export async function readInvitation(input) {
    const chunks = [];
    let length = 0;
    for await (const chunk of input) {
        const bytes = Buffer.from(chunk);
        length += bytes.length;
        if (length > 16 * 1024) throw new Error('Invitation exceeds the size limit.');
        chunks.push(bytes);
    }
    let invitation;
    try {
        invitation = JSON.parse(Buffer.concat(chunks).toString('utf8').trim());
    } catch {
        throw new Error('Expected a JSON invitation on standard input.');
    }
    if (
        invitation?.version !== 1 ||
        !loopbackPort(invitation.url) ||
        !/^[a-f0-9]{64}$/.test(invitation.code) ||
        !Number.isSafeInteger(invitation.expiresAt)
    ) {
        throw new Error('Invalid Mundam invitation.');
    }
    return invitation;
}

/** Validates a stored or invited certificate independently of endpoint discovery. */
export function trustedIdentity({ certificateDer, publicKeySha256 }) {
    if (
        typeof certificateDer !== 'string' ||
        certificateDer.length > 16 * 1024 ||
        typeof publicKeySha256 !== 'string'
    )
        throw new Error('Invalid Mundam server identity.');
    const certificate = new X509Certificate(Buffer.from(certificateDer, 'base64'));
    if (publicKeyPin(certificate) !== publicKeySha256) {
        throw new Error('The certificate does not match its pinned public key.');
    }
    return { certificate: certificate.toString(), pin: publicKeySha256 };
}

/** Waits for desktop approval and returns a credential for the original pinned identity. */
export async function pairImporter(invitation, clientName, scopes, onRequested) {
    if (Date.now() >= invitation.expiresAt * 1000) throw new Error('Invitation expired.');
    const identity = trustedIdentity(invitation);
    const server = { port: loopbackPort(invitation.url) };
    const post = (path, payload) =>
        exchange(server, identity, 'POST', path, {
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify(payload)
        });
    const request = await post('/v1/pairing/requests', {
        code: invitation.code,
        clientName,
        scopes
    });
    expectStatus(request, 202);
    onRequested?.();
    while (Date.now() < invitation.expiresAt * 1000) {
        await pause(2000);
        const response = await post('/v1/pairing/claims', request.data);
        if (response.status === 401) continue;
        if (response.status === 429) {
            await pause(15_000);
            continue;
        }
        const credential = expectStatus(response, 201);
        const profile = {
            version: 1,
            certificateDer: invitation.certificateDer,
            publicKeySha256: identity.pin,
            clientId: credential?.clientId,
            bearer: credential?.bearer,
            expiresAt: credential?.expiresAt
        };
        validateProfile(profile);
        return { profile, identity, server };
    }
    throw new Error('The desktop did not approve the request before expiration.');
}

/** Checks the local profile envelope before it can authorize any request. */
export function validateProfile(profile) {
    if (
        profile?.version !== 1 ||
        typeof profile.clientId !== 'string' ||
        !/^[a-f0-9-]{36}$/.test(profile.clientId) ||
        !/^[a-f0-9]{64}$/.test(profile.bearer) ||
        !Number.isSafeInteger(profile.expiresAt)
    ) {
        throw new Error('Invalid Mundam client profile.');
    }
    return trustedIdentity(profile);
}

/** Creates a session that retries idempotent operations after a local port change. */
export function createSession(profile, discoveryPath) {
    const identity = validateProfile(profile);
    if (Date.now() >= profile.expiresAt * 1000) throw new Error('Credential expired. Pair again.');
    return async (method, path, options = {}) => {
        const find = async () => {
            const server = await readDiscovery(discoveryPath, identity.pin);
            if (!server) throw new Error('Open Mundam and enable the API for the paired identity.');
            return server;
        };
        const send = server =>
            exchange(server, identity, method, path, { ...options, token: profile.bearer });
        const server = await find();
        try {
            return await send(server);
        } catch (error) {
            if (
                ![
                    'ECONNREFUSED',
                    'ECONNRESET',
                    'EPIPE',
                    'ETIMEDOUT',
                    'ERR_STREAM_PREMATURE_CLOSE'
                ].includes(error.code)
            )
                throw error;
            return send(await find());
        }
    };
}

/** Extracts the response without displaying untrusted server text. */
export function expectStatus(response, expected) {
    if (response.status === expected) return response.data;
    const code = response.data?.error?.code;
    const suffix = typeof code === 'string' && /^[A-Z_]{1,64}$/.test(code) ? ` (${code})` : '';
    const credentialHint =
        response.status === 401 ? ' Check whether the paired credential is still active.' : '';
    throw new Error(`Mundam returned HTTP ${response.status}${suffix}.${credentialHint}`);
}
