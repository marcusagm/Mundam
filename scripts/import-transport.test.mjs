import assert from 'node:assert/strict';
import { randomBytes } from 'node:crypto';
import { once } from 'node:events';
import { mkdtemp, rm, rmdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import test from 'node:test';
import {
    buildProbe,
    createIdentity,
    digest,
    exchange,
    openRequest,
    startServer
} from './lib/import-transport-probe.mjs';

test('authenticated loopback upload resumes from its durable receipt after process loss', async context => {
    const executable = buildProbe();
    const directory = await mkdtemp(join(tmpdir(), 'mundam-transport-'));
    const database = join(directory, 'staging.sqlite');
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
    const { ownerToken, otherToken } = identity.config;
    server = await startServer(executable, database, identity);
    const bytes = randomBytes(2 * 256 * 1024 + 97);
    const manifest = {
        folderId: 'probe-folder',
        items: [{ itemId: 'photo', fileName: 'photo.png', byteLength: bytes.length }]
    };
    const admit = (value, token = ownerToken, extra = {}) =>
        exchange(server, identity, 'POST', '/v1/imports', {
            token,
            body: JSON.stringify(value),
            headers: {
                'content-type': 'application/json',
                'idempotency-key': 'resume-fixture',
                ...extra
            }
        });
    await assert.rejects(
        exchange(server, identity, 'GET', '/v1/imports/none', {
            token: ownerToken,
            pin: 'invalid-pin'
        }),
        /Server key pin mismatch/
    );
    assert.equal((await admit(manifest, '')).status, 401);
    assert.equal((await admit(manifest, 'unpaired')).status, 401);
    assert.equal(
        (await admit(manifest, ownerToken, { origin: 'https://example.org' })).status,
        403
    );
    // Keep the valid TLS identity while omitting the required ephemeral port in HTTP Host.
    assert.equal((await admit(manifest, ownerToken, { host: '127.0.0.1' })).status, 403);
    assert.equal((await admit({ ...manifest, folderId: 'outside-grant' })).status, 403);
    assert.equal((await admit({ ...manifest, folderId: null })).status, 409);
    const accepted = await admit(manifest);
    assert.equal(accepted.status, 201);
    const { batchId, receiptPath, partSizeBytes } = accepted.data;
    assert.equal(receiptPath, '/v1/imports/' + batchId);
    assert.equal(partSizeBytes, 256 * 1024);
    const part = number => bytes.subarray(number * partSizeBytes, (number + 1) * partSizeBytes);
    const route = number => `${receiptPath}/items/photo/parts/${number}`;
    const headers = body => ({
        'content-type': 'application/octet-stream',
        'x-part-sha256': digest(body)
    });
    const put = (number, body, token = ownerToken, extra = {}) =>
        exchange(server, identity, 'PUT', route(number), {
            token,
            body,
            headers: { ...headers(body), ...extra }
        });
    const receipt = token => exchange(server, identity, 'GET', receiptPath, { token });
    const hidden = await receipt(otherToken);
    const absent = await exchange(server, identity, 'GET', '/v1/imports/absent', {
        token: otherToken
    });
    assert.equal(hidden.status, 404);
    assert.equal(absent.status, hidden.status);
    assert.deepEqual(hidden.data.error, absent.data.error);
    assert.equal(hidden.data.error.code, 'NOT_FOUND');
    assert.equal((await put(0, part(0), otherToken)).status, 404);
    const otherBatch = await admit(manifest, otherToken);
    assert.equal(otherBatch.status, 201);
    assert.notEqual(otherBatch.data.batchId, batchId);
    assert.equal(
        (await put(2, part(2), ownerToken, { 'x-part-sha256': '0'.repeat(64) })).status,
        422
    );
    assert.equal((await put(2, part(2).subarray(1))).status, 422);
    assert.equal((await put(3, Buffer.alloc(0))).status, 422);
    assert.equal((await put(0, Buffer.alloc(partSizeBytes + 1))).status, 413);
    assert.deepEqual((await receipt(ownerToken)).data.confirmedParts, []);

    const ackStart = performance.now();
    assert.equal((await put(1, part(1))).status, 204);
    const ackMs = performance.now() - ackStart;
    const partial = openRequest(server.port, identity, 'PUT', route(0), {
        authorization: `Bearer ${ownerToken}`,
        ...headers(part(0)),
        expect: '100-continue',
        'content-length': partSizeBytes
    });
    // Leave the declared body incomplete when the controller terminates the server.
    const interrupted = new Promise(resolve => partial.once('error', resolve));
    try {
        const receiving = once(partial, 'continue');
        partial.flushHeaders();
        await receiving;
        await Promise.race([
            new Promise((resolve, reject) =>
                partial.write(part(0).subarray(0, 4096), error =>
                    error ? reject(error) : resolve()
                )
            ),
            interrupted.then(error => Promise.reject(error))
        ]);
        await server.stop();
        server = undefined;
        assert.ok((await interrupted) instanceof Error);
    } finally {
        partial.destroy();
    }
    const restartStart = performance.now();
    server = await startServer(executable, database, identity);
    assert.deepEqual((await admit(manifest)).data, accepted.data);
    assert.equal(
        (await admit({ ...manifest, items: [{ ...manifest.items[0], fileName: 'other.png' }] }))
            .status,
        409
    );
    const resumed = await receipt(ownerToken);
    assert.equal(resumed.status, 200);
    assert.deepEqual(resumed.data.confirmedParts, [{ itemId: 'photo', partNumber: 1 }]);
    const restartMs = performance.now() - restartStart;
    assert.equal((await put(1, part(1))).status, 204);
    assert.equal((await put(1, Buffer.alloc(partSizeBytes))).status, 409);
    const confirmed = new Set(resumed.data.confirmedParts.map(entry => entry.partNumber));
    const missing = Array.from(
        { length: Math.ceil(bytes.length / partSizeBytes) },
        (_, number) => number
    ).filter(number => !confirmed.has(number));
    assert.deepEqual(missing, [0, 2]);
    const writes = await Promise.all(
        [...missing, missing[0]].map(number => put(number, part(number)))
    );
    assert.deepEqual(
        writes.map(response => response.status),
        [204, 204, 204]
    );
    const completed = (await receipt(ownerToken)).data;
    assert.deepEqual(completed.items, [{ itemId: 'photo', state: 'ready' }]);
    assert.deepEqual(
        completed.confirmedParts.map(entry => entry.partNumber),
        [0, 1, 2]
    );
    await server.stop();
    server = undefined;
    const stored = new DatabaseSync(database, { readOnly: true });
    try {
        const rows = stored
            .prepare('SELECT content FROM parts WHERE batch = ? ORDER BY number')
            .all(batchId);
        assert.equal(rows.length, 3);
        assert.equal(digest(Buffer.concat(rows.map(row => row.content))), digest(bytes));
    } finally {
        stored.close();
    }
    context.diagnostic(
        `payloadBytes=${bytes.length} durableParts=3 firstAckMs=${ackMs.toFixed(1)} restartAndReadMs=${restartMs.toFixed(1)}`
    );
});
