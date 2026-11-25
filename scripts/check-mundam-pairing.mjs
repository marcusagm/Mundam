import { X509Certificate } from 'node:crypto';
import { setTimeout as pause } from 'node:timers/promises';
import { digest, exchange } from './lib/import-transport-probe.mjs';
import { loopbackPort, readDiscovery, windowsDiscoveryPath } from './lib/import-discovery.mjs';
import { checkImportStaging } from './lib/check-import-staging.mjs';
import { checkImportPublication } from './lib/check-import-publication.mjs';

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
    const port = loopbackPort(invitation.url);
    if (!port) {
        throw new Error('Expected an HTTPS IPv4 loopback endpoint with an explicit port.');
    }
    if (Date.now() >= invitation.expiresAt * 1000) throw new Error('Invitation expired.');
    const certificate = new X509Certificate(Buffer.from(invitation.certificateDer, 'base64'));
    const pin = digest(certificate.publicKey.export({ format: 'der', type: 'spki' }));
    if (pin !== invitation.publicKeySha256)
        throw new Error('The certificate does not match the invitation fingerprint.');
    const identity = { certificate: certificate.toString(), pin };
    const server = { port };
    const post = (path, payload) =>
        exchange(server, identity, 'POST', path, {
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify(payload)
        });
    const verifyStaging = process.argv.includes('--verify-staging');
    const verifyPublication = process.argv.includes('--verify-publication');
    const scopes = verifyStaging || verifyPublication
        ? ['folders.read', 'assets.import']
        : ['folders.read', 'tags.read'];
    const request = await post('/v1/pairing/requests', {
        code: invitation.code,
        clientName: 'Mundam pairing check',
        scopes
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
        const roots = capabilities.grants.includes('folders.read')
            ? (await read('/v1/folders')).items
            : [];
        if (capabilities.grants.includes('folders.read'))
            console.log(`Visible roots: ${roots.length}`);
        if (capabilities.grants.includes('tags.read'))
            console.log(`Tags on the first page: ${(await read('/v1/tags')).items.length}`);
        if (verifyStaging) {
            if (
                !capabilities.uploadsAvailable ||
                !capabilities.grants.includes('assets.import') ||
                !roots.length
            ) {
                throw new Error(
                    'Staging check requires folder read and asset import grants for an indexed root.'
                );
            }
            await checkImportStaging(server, identity, token, roots[0].id);
        }
        if (verifyPublication) {
            if (
                !capabilities.publicationAvailable ||
                !capabilities.mediaTypes.includes('image/png') ||
                !capabilities.grants.includes('assets.import') ||
                roots.length !== 1
            ) {
                throw new Error('Publication check requires PNG support and exactly one granted test root.');
            }
            await checkImportPublication(server, identity, token, roots[0].id);
        }
        if (process.argv.includes('--verify-reconnect')) {
            await verifyReconnect(server, identity, token);
        }
        return;
    }
    throw new Error('The request was not approved before the invitation expired.');
}

async function verifyReconnect(server, identity, token) {
    const path = windowsDiscoveryPath();
    const current = await readDiscovery(path, identity.pin);
    if (!current || current.port !== server.port)
        throw new Error('Discovery did not report the active endpoint.');
    console.log('Discovery verified. Disable and enable the API in Mundam to check reconnection.');
    const deadline = Date.now() + 120_000;
    while (Date.now() < deadline) {
        await pause(1000);
        const discovered = await readDiscovery(path, identity.pin);
        if (!discovered || discovered.instanceId === current.instanceId) continue;
        const response = await exchange(discovered, identity, 'GET', '/v1/capabilities', { token });
        if (response.status !== 200) throw new Error(`Reconnect returned HTTP ${response.status}.`);
        console.log('Reconnected using discovery and the original verified identity.');
        return;
    }
    throw new Error('No new instance was discovered within two minutes.');
}

try {
    await checkPairing();
} catch (error) {
    console.error(error instanceof Error ? error.message : 'Pairing check failed.');
    process.exitCode = 1;
}
