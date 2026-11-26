import assert from 'node:assert/strict';
import { createPrivateKey, randomUUID } from 'node:crypto';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import https from 'node:https';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { digest } from './lib/import-http.mjs';
import { loadJob, prepareJob, runJob, saveJob } from './lib/import-job.mjs';
import { createSession } from './lib/import-session.mjs';
import { createIdentity } from './lib/import-transport-probe.mjs';

test('a disconnected job resumes on a new pinned port and sends only missing parts', async () => {
    const directory = await mkdtemp(join(tmpdir(), 'mundam-import-client-'));
    const servers = [];
    try {
        const identity = await createIdentity(directory);
        const profile = {
            version: 1,
            clientId: randomUUID(),
            bearer: identity.config.ownerToken,
            expiresAt: Math.floor(Date.now() / 1000) + 600,
            certificateDer: identity.config.certificateDer,
            publicKeySha256: identity.pin
        };
        const key = createPrivateKey({
            key: Buffer.from(identity.config.privateKeyDer, 'base64'),
            format: 'der',
            type: 'pkcs8'
        }).export({ type: 'pkcs8', format: 'pem' });
        const discovery = join(directory, 'import-endpoint.json');
        const source = join(directory, 'cover.png');
        const localJob = join(directory, 'import-job.json');
        const partBytes = 256 * 1024;
        const bytes = Buffer.alloc(partBytes + 8, 42);
        Buffer.from('89504e470d0a1a0a', 'hex').copy(bytes);
        await writeFile(source, bytes);
        const job = await prepareJob({
            paths: [source],
            folderId: 'test-root',
            tagIds: ['proof'],
            profile
        });
        await saveJob(localJob, job);
        await assert.rejects(saveJob(localJob, job), { code: 'EEXIST' });
        await assert.rejects(
            loadJob(localJob, { ...profile, clientId: randomUUID() }),
            /another paired client/
        );
        const batchId = randomUUID();
        const assetId = randomUUID();
        const receiptPath = `/v1/imports/${batchId}`;
        const parts = [];
        let commitCount = 0;
        let persistedKey;
        let observedFailure;
        async function serve(disconnect, certificate = identity.certificate, privateKey = key) {
            const server = https.createServer(
                { cert: certificate, key: privateKey },
                async (request, response) => {
                    try {
                        assert.equal(request.headers.authorization, `Bearer ${profile.bearer}`);
                        const chunks = [];
                        for await (const chunk of request) chunks.push(chunk);
                        const body = Buffer.concat(chunks);
                        let status = 200;
                        let json;
                        if (request.url === '/v1/imports') {
                            const requestKey = request.headers['idempotency-key'];
                            assert.equal(requestKey, job.key);
                            persistedKey ??= requestKey;
                            assert.equal(requestKey, persistedKey);
                            assert.deepEqual(JSON.parse(body), job.manifest);
                            status = 201;
                            json = { batchId, partSizeBytes: partBytes, receiptPath };
                        } else if (request.method === 'PUT') {
                            const number = Number(request.url.split('/').at(-1));
                            if (disconnect && number === 1) {
                                request.socket.destroy();
                                return;
                            }
                            assert.equal(number, parts.length);
                            assert.deepEqual(
                                body,
                                bytes.subarray(number * partBytes, (number + 1) * partBytes)
                            );
                            assert.equal(request.headers['x-part-sha256'], digest(body));
                            parts.push(number);
                            status = 204;
                        } else if (request.url === `${receiptPath}/commit`) {
                            assert.deepEqual(parts, [0, 1]);
                            commitCount++;
                            status = 202;
                            json = { state: 'publishing' };
                        } else {
                            const afterFirst = request.url === `${receiptPath}?cursor=0`;
                            assert(request.url === receiptPath || afterFirst);
                            json = {
                                batchId,
                                state: commitCount
                                    ? 'committed'
                                    : parts.length === 2
                                      ? 'ready'
                                      : 'receiving',
                                items: [
                                    {
                                        itemId: 'item-1',
                                        state: commitCount ? 'committed' : 'receiving',
                                        ...(commitCount ? { assetId, fileName: 'cover.png' } : {})
                                    }
                                ],
                                confirmedParts:
                                    commitCount || afterFirst
                                        ? []
                                        : parts.map(partNumber => ({
                                              itemId: 'item-1',
                                              partNumber
                                          })),
                                nextCursor:
                                    !afterFirst && !commitCount && parts.length === 1 ? '0' : null
                            };
                        }
                        response.writeHead(status, { 'content-type': 'application/json' });
                        response.end(json ? JSON.stringify(json) : undefined);
                    } catch (error) {
                        observedFailure = error;
                        response.writeHead(500).end();
                    }
                }
            );
            servers.push(server);
            await new Promise((resolve, reject) => {
                server.once('error', reject);
                server.listen(0, '127.0.0.1', resolve);
            });
            const port = server.address().port;
            await writeFile(
                discovery,
                JSON.stringify({
                    version: 1,
                    url: `https://127.0.0.1:${port}`,
                    publicKeySha256: identity.pin,
                    instanceId: randomUUID()
                })
            );
            return port;
        }

        const first = await serve(true);
        await assert.rejects(runJob(job, createSession(profile, discovery)), {
            code: 'ECONNRESET'
        });
        assert.deepEqual(parts, [0]);
        const second = await serve(false);
        assert.notEqual(first, second);
        const resumed = await loadJob(localJob, profile);
        await writeFile(source, Buffer.concat([bytes.subarray(0, -1), Buffer.from([0])]));
        await assert.rejects(
            runJob(resumed, createSession(profile, discovery)),
            /Source bytes changed/
        );
        await writeFile(source, bytes);
        const receipt = await runJob(resumed, createSession(profile, discovery));
        assert.equal(receipt.items[0].assetId, assetId);
        assert.deepEqual(await runJob(resumed, createSession(profile, discovery)), receipt);
        assert.equal(commitCount, 1);
        assert.ifError(observedFailure);

        const otherIdentity = await createIdentity(directory);
        const otherKey = createPrivateKey({
            key: Buffer.from(otherIdentity.config.privateKeyDer, 'base64'),
            format: 'der',
            type: 'pkcs8'
        }).export({ type: 'pkcs8', format: 'pem' });
        await serve(false, otherIdentity.certificate, otherKey);
        await assert.rejects(runJob(resumed, createSession(profile, discovery)), error =>
            /certificate/i.test(error.message)
        );
        assert.equal(commitCount, 1);
        assert.ifError(observedFailure);
    } finally {
        for (const server of servers) await new Promise(resolve => server.close(resolve));
        await rm(directory, { recursive: true, force: true });
    }
});
