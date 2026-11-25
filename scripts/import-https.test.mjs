import assert from 'node:assert/strict';
import { once } from 'node:events';
import { mkdtemp, rm, rmdir } from 'node:fs/promises';
import { connect } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import {
    buildProbe,
    createIdentity,
    exchange,
    startServer
} from './lib/import-transport-probe.mjs';

test('pairs and reads the catalog over pinned HTTPS and drains on shutdown', async context => {
    const executable = buildProbe('import_https');
    const directory = await mkdtemp(join(tmpdir(), 'mundam-https-'));
    const database = join(directory, 'catalog.sqlite');
    let server;
    context.after(async () => {
        await server?.stop();
        for (const file of [
            database,
            database + '-wal',
            database + '-shm',
            join(directory, 'test-key.pem')
        ]) {
            await rm(file, { force: true });
        }
        await rmdir(directory);
    });
    const identity = await createIdentity(directory);
    server = await startServer(executable, database, identity, true);
    assert.equal(server.readyData.serverKey, identity.pin);
    const request = (path, options) => exchange(server, identity, 'GET', path, options);
    const post = (path, payload) =>
        exchange(server, identity, 'POST', path, {
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify(payload)
        });
    await assert.rejects(
        request('/v1/folders', { pin: 'incorrect-key-pin' }),
        /Server key pin mismatch/
    );
    const invitation = await post('/v1/pairing/requests', {
        code: server.readyData.code,
        clientName: 'HTTPS test client',
        scopes: ['folders.read', 'tags.read']
    });
    assert.equal(invitation.status, 202);
    const claim = invitation.data;
    assert.equal((await post('/v1/pairing/claims', claim)).status, 401);
    assert.deepEqual(
        await server.desktop({
            action: 'approve',
            id: claim.requestId,
            scopes: ['folders.read', 'tags.read']
        }),
        { ok: true }
    );
    const credential = await post('/v1/pairing/claims', claim);
    assert.equal(credential.status, 201);
    const token = credential.data.bearer;
    const folders = await request('/v1/folders', { token });
    assert.equal(folders.status, 200);
    assert.deepEqual(
        folders.data.items.map(folder => folder.id),
        ['references']
    );
    assert.equal(Object.hasOwn(folders.data.items[0], 'path'), false);
    const tags = await request('/v1/tags?query=portrait', { token });
    assert.equal(tags.status, 200);
    assert.deepEqual(tags.data.items[0].labels, ['Portrait']);

    const stalled = connect({ host: '127.0.0.1', port: server.port });
    const socketClosed = new Promise(resolve => stalled.once('close', resolve));
    try {
        await once(stalled, 'connect');
        stalled.on('error', () => {});
        await server.shutdown();
        server = undefined;
        await socketClosed;
    } finally {
        stalled.destroy();
    }
    server = await startServer(executable, database, identity, true);
    assert.equal((await request('/v1/folders', { token })).status, 200);
    await server.shutdown();
    server = undefined;

    const replacement = await createIdentity(directory);
    server = await startServer(executable, database, replacement, true);
    assert.equal(server.readyData.serverKey, replacement.pin);
    assert.notEqual(replacement.pin, identity.pin);
    assert.equal(
        (await exchange(server, replacement, 'GET', '/v1/folders', { token })).status,
        401
    );
    await server.shutdown();
    server = undefined;
});
