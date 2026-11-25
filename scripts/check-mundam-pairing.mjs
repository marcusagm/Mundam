import { X509Certificate } from 'node:crypto';
import { setTimeout as pause } from 'node:timers/promises';
import { digest, exchange } from './lib/import-transport-probe.mjs';

async function readInvitation() {
    const chunks = [];
    let length = 0;
    for await (const chunk of process.stdin) {
        length += chunk.length;
        if (length > 16 * 1024) throw new Error('Invitation exceeds the size limit.');
        chunks.push(chunk);
    }
    let invitation;
    try {
        invitation = JSON.parse(Buffer.concat(chunks).toString('utf8').trim());
    } catch {
        throw new Error('Expected a JSON invitation on standard input.');
    }
    if (
        invitation?.version !== 1 ||
        typeof invitation.url !== 'string' ||
        !/^[a-f0-9]{64}$/.test(invitation.code) ||
        !Number.isSafeInteger(invitation.expiresAt)
    ) {
        throw new Error('Invalid invitation.');
    }
    return invitation;
}

async function checkPairing() {
    const invitation = await readInvitation();
    const endpoint = new URL(invitation.url);
    if (!/^https:\/\/127\.0\.0\.1:[0-9]{1,5}\/?$/.test(invitation.url) || !endpoint.port) {
        throw new Error('Expected an HTTPS IPv4 loopback endpoint with an explicit port.');
    }
    if (Date.now() >= invitation.expiresAt * 1000) throw new Error('Invitation expired.');
    const certificate = new X509Certificate(Buffer.from(invitation.certificateDer, 'base64'));
    const pin = digest(certificate.publicKey.export({ format: 'der', type: 'spki' }));
    if (pin !== invitation.publicKeySha256)
        throw new Error('The certificate does not match the invitation fingerprint.');
    const identity = { certificate: certificate.toString(), pin };
    const server = { port: Number(endpoint.port) };
    const post = (path, payload) =>
        exchange(server, identity, 'POST', path, {
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify(payload)
        });
    const request = await post('/v1/pairing/requests', {
        code: invitation.code,
        clientName: 'Mundam pairing check',
        scopes: ['folders.read', 'tags.read']
    });
    if (request.status !== 202) throw new Error(`Pairing request returned HTTP ${request.status}.`);
    console.log('Pairing requested. Refresh requests in Mundam, select the grants, and approve.');
    while (Date.now() < invitation.expiresAt * 1000) {
        await pause(3000);
        const claim = await post('/v1/pairing/claims', request.data);
        if (claim.status === 401) continue;
        if (claim.status === 429) {
            await pause(15000);
            continue;
        }
        if (claim.status !== 201) throw new Error(`Pairing claim returned HTTP ${claim.status}.`);
        const token = claim.data.bearer;
        const read = async path => {
            const response = await exchange(server, identity, 'GET', path, { token });
            if (response.status !== 200)
                throw new Error(`Catalog read returned HTTP ${response.status}.`);
            return response.data;
        };
        const capabilities = await read('/v1/capabilities');
        console.log('Paired. Granted scopes:', capabilities.grants.join(', '));
        for (const [scope, route, label] of [
            ['folders.read', '/v1/folders', 'Visible roots'],
            ['tags.read', '/v1/tags', 'Tags on the first page']
        ]) {
            if (capabilities.grants.includes(scope))
                console.log(`${label}: ${(await read(route)).items.length}`);
        }
        return;
    }
    throw new Error('The request was not approved before the invitation expired.');
}

try {
    await checkPairing();
} catch (error) {
    console.error(error instanceof Error ? error.message : 'Pairing check failed.');
    process.exitCode = 1;
}
