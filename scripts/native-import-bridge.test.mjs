import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { Readable, Writable } from 'node:stream';
import { test } from 'node:test';
import { digest } from './lib/import-http.mjs';
import { acceptNativeMessage, runNativeBridge } from './lib/native-import-bridge.mjs';
import { maxFrameBytes, readNativeFrames, writeNativeFrame } from './lib/native-message-frames.mjs';

function capture() {
    const chunks = [];
    return {
        output: new Writable({
            write(chunk, _, done) {
                chunks.push(chunk);
                done();
            }
        }),
        bytes: () => Buffer.concat(chunks)
    };
}

test('bridge serializes split frames, sends binary parts and resumes by receipt', async () => {
    const batchId = randomUUID();
    const key = randomUUID();
    const raw = Buffer.alloc(256 * 1024, 0x42);
    const inputFrames = capture();
    const frame = (id, op, options = {}) =>
        writeNativeFrame(inputFrames.output, { version: 1, id, op, ...options });
    await frame('start', 'admit', {
        key,
        manifest: {
            folderId: 'root',
            items: [{ itemId: 'image', fileName: 'image.png', byteLength: raw.length }]
        }
    });
    await frame('block', 'part', {
        batchId,
        itemId: 'image',
        part: 0,
        base64: raw.toString('base64')
    });
    await frame('resume', 'receipt', { batchId, cursor: '0' });
    await frame('commit', 'commit', { batchId });
    const bytes = inputFrames.bytes();
    const response = capture();
    const calls = [];
    const send = async (method, path, options) => {
        calls.push([method, path, options]);
        if (method === 'PUT') return { status: 204, data: null };
        if (method === 'GET')
            return { status: 200, data: { confirmedParts: [{ itemId: 'image', partNumber: 0 }] } };
        return { status: path.endsWith('/commit') ? 202 : 201, data: { batchId } };
    };
    await runNativeBridge(
        Readable.from([bytes.subarray(0, 2), bytes.subarray(2, 9), bytes.subarray(9)]),
        response.output,
        send
    );
    const replies = [];
    for await (const reply of readNativeFrames(Readable.from([response.bytes()])))
        replies.push(reply);
    assert.deepEqual(
        replies.map(({ id, status }) => [id, status]),
        [
            ['start', 201],
            ['block', 204],
            ['resume', 200],
            ['commit', 202]
        ]
    );
    assert.equal(calls[0][2].headers['idempotency-key'], key);
    assert.deepEqual(calls[1], [
        'PUT',
        `/v1/imports/${batchId}/items/image/parts/0`,
        {
            headers: { 'content-type': 'application/octet-stream', 'x-part-sha256': digest(raw) },
            body: raw
        }
    ]);
    assert.deepEqual(calls[2], ['GET', `/v1/imports/${batchId}?cursor=0`, undefined]);
});

test('bridge rejects invalid framing and transport overrides without contacting the API', async () => {
    const oversized = Buffer.alloc(4);
    oversized.writeUInt32LE(maxFrameBytes + 1);
    const send = async () => {
        throw new Error('The transport must not be called');
    };
    await assert.rejects(
        runNativeBridge(Readable.from([oversized]), capture().output, send),
        /length/
    );
    await assert.rejects(
        runNativeBridge(Readable.from([Buffer.from([1, 0, 0])]), capture().output, send),
        /Truncated/
    );
    for (const body of [
        { op: 'pair', token: 'secret' },
        { op: 'folders', url: 'https://outside.example/' },
        { op: 'receipt', batchId: '../credentials/rotate' },
        { op: 'part', batchId: randomUUID(), itemId: 'image', part: 0, base64: 'eA' }
    ]) {
        const result = await acceptNativeMessage({ version: 1, id: 'test', ...body }, send);
        assert.deepEqual(result, { version: 1, id: 'test', error: 'INVALID_MESSAGE' });
    }
});
