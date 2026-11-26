import { randomUUID } from 'node:crypto';
import { open } from 'node:fs/promises';
import { basename, extname, isAbsolute, resolve } from 'node:path';
import { setTimeout as pause } from 'node:timers/promises';
import { digest } from './import-http.mjs';
import { readBounded } from './import-client-storage.mjs';
import { expectStatus } from './import-session.mjs';

const maxItemBytes = 16 * 1024 * 1024;
const maxBatchBytes = 32 * 1024 * 1024;
const maxItems = 8;
const maxTags = 32;
const jobLifetime = 24 * 60 * 60 * 1000;
const pngSignature = Buffer.from('89504e470d0a1a0a', 'hex');
const terminal = item =>
    item.state === 'committed' ||
    item.state === 'cancelled' ||
    (item.state === 'failed' && item.error?.retryable === false);

/** Creates the immutable import manifest and source fingerprints before admission. */
export async function prepareJob({ paths, folderId, tagIds, sourceUrl, profile }) {
    if (paths.length < 1 || paths.length > maxItems || tagIds.length > maxTags) {
        throw new Error('Select one to eight files and at most 32 tags.');
    }
    if (sourceUrl) {
        const origin = new URL(sourceUrl);
        if (!['http:', 'https:'].includes(origin.protocol) || origin.username || origin.password) {
            throw new Error('The source URL must use HTTP(S) without credentials.');
        }
    }
    let total = 0;
    const sources = [];
    const items = [];
    for (const file of paths) {
        const path = resolve(file);
        const fileName = basename(path);
        if (extname(fileName).toLowerCase() !== '.png')
            throw new Error('The Windows pilot imports PNG files.');
        const bytes = await readBounded(path, maxItemBytes);
        if (!bytes.subarray(0, pngSignature.length).equals(pngSignature))
            throw new Error('A selected file has no PNG signature.');
        total += bytes.length;
        if (total > maxBatchBytes) throw new Error('Selected files exceed the batch size limit.');
        sources.push({ path, sha256: digest(bytes) });
        items.push({
            itemId: `item-${items.length + 1}`,
            fileName,
            byteLength: bytes.length,
            ...(sourceUrl ? { sourceUrl } : {})
        });
    }
    return {
        version: 1,
        key: randomUUID(),
        createdAt: Date.now(),
        clientId: profile.clientId,
        pin: profile.publicKeySha256,
        manifest: { ...(folderId ? { folderId } : {}), tagIds, items },
        sources
    };
}

/** Saves a new job exclusively and synchronizes it before sending the first request. */
export async function saveJob(path, job) {
    const file = await open(path, 'wx', 0o600);
    try {
        await file.writeFile(JSON.stringify(job, null, 2));
        await file.sync();
    } finally {
        await file.close();
    }
}

/** Reads a local job, rejecting expired or foreign credentials before replay. */
export async function loadJob(path, profile) {
    const job = JSON.parse((await readBounded(path, 64 * 1024)).toString('utf8'));
    const items = job?.manifest?.items;
    if (
        job?.version !== 1 ||
        !/^[a-f0-9-]{36}$/.test(job.key) ||
        job.clientId !== profile.clientId ||
        job.pin !== profile.publicKeySha256 ||
        !Number.isSafeInteger(job.createdAt) ||
        !Array.isArray(job.sources) ||
        !Array.isArray(items) ||
        items.length < 1 ||
        items.length > maxItems ||
        items.length !== job.sources.length ||
        items.some(
            (item, index) =>
                item?.itemId !== `item-${index + 1}` ||
                !Number.isSafeInteger(item.byteLength) ||
                item.byteLength < 1 ||
                item.byteLength > maxItemBytes ||
                typeof job.sources[index]?.path !== 'string' ||
                !isAbsolute(job.sources[index].path) ||
                !/^[a-f0-9]{64}$/.test(job.sources[index]?.sha256)
        )
    ) {
        throw new Error('Invalid job or a job belonging to another paired client.');
    }
    const age = Date.now() - job.createdAt;
    if (age < 0 || age >= jobLifetime) {
        throw new Error('The reference client only replays a job within 24 hours of its creation.');
    }
    return job;
}

async function confirmedParts(readReceipt, path, receipt) {
    const confirmed = new Set();
    let previous = -1;
    for (let page = 0; page < 16; page++) {
        for (const { itemId, partNumber } of receipt.confirmedParts) {
            if (Number.isSafeInteger(partNumber) && partNumber >= 0) {
                confirmed.add(`${itemId}/${partNumber}`);
            }
        }
        if (receipt.nextCursor === null) return confirmed;
        const cursor = receipt.nextCursor;
        if (typeof cursor !== 'string' || !/^\d{1,4}$/.test(cursor) || Number(cursor) <= previous) {
            throw new Error('Invalid or repeated part receipt cursor.');
        }
        previous = Number(cursor);
        receipt = await readReceipt(`${path}?cursor=${cursor}`);
    }
    throw new Error('Too many part receipt pages.');
}

/** Resumes a job using the same reservation key and only the missing confirmed parts. */
export async function runJob(job, send, onProgress = () => {}) {
    const playback = expectStatus(
        await send('POST', '/v1/imports', {
            headers: { 'content-type': 'application/json', 'idempotency-key': job.key },
            body: JSON.stringify(job.manifest)
        }),
        201
    );
    const { batchId, partSizeBytes } = playback ?? {};
    if (
        typeof batchId !== 'string' ||
        !/^[a-f0-9-]{36}$/.test(batchId) ||
        partSizeBytes !== 256 * 1024 ||
        playback.receiptPath !== `/v1/imports/${batchId}`
    ) {
        throw new Error('Unexpected Mundam upload reservation.');
    }
    const path = playback.receiptPath;
    const readReceipt = async route => {
        const receipt = expectStatus(await send('GET', route), 200);
        const expected = job.manifest.items;
        if (
            receipt?.batchId !== batchId ||
            !Array.isArray(receipt.items) ||
            receipt.items.length !== expected.length ||
            !expected.every(item =>
                receipt.items.some(candidate => candidate?.itemId === item.itemId)
            ) ||
            !Array.isArray(receipt.confirmedParts)
        ) {
            throw new Error('Invalid Mundam receipt.');
        }
        return receipt;
    };
    const current = await readReceipt(path);
    if (current.items.every(terminal)) return current;
    const done = await confirmedParts(readReceipt, path, current);
    for (const [index, item] of job.manifest.items.entries()) {
        const observed = current.items.find(candidate => candidate.itemId === item.itemId);
        if (terminal(observed)) continue;
        const parts = Math.ceil(item.byteLength / partSizeBytes);
        const missing = [];
        for (let part = 0; part < parts; part++) {
            if (!done.has(`${item.itemId}/${part}`)) missing.push(part);
        }
        if (!missing.length) continue;
        const source = job.sources[index];
        const bytes = await readBounded(source.path, maxItemBytes);
        if (bytes.length !== item.byteLength || digest(bytes) !== source.sha256) {
            throw new Error(
                `Source bytes changed for ${item.fileName}. Restore the original file to resume.`
            );
        }
        for (const part of missing) {
            const body = bytes.subarray(part * partSizeBytes, (part + 1) * partSizeBytes);
            expectStatus(
                await send('PUT', `${path}/items/${item.itemId}/parts/${part}`, {
                    headers: {
                        'content-type': 'application/octet-stream',
                        'x-part-sha256': digest(body)
                    },
                    body
                }),
                204
            );
            onProgress(item.fileName, part + 1, parts);
        }
    }
    expectStatus(await send('POST', `${path}/commit`), 202);
    const deadline = Date.now() + 30_000;
    while (Date.now() < deadline) {
        const receipt = await readReceipt(path);
        if (
            receipt.items.every(terminal) ||
            (receipt.state !== 'publishing' && receipt.items.some(item => item.error?.retryable))
        )
            return receipt;
        await pause(300);
    }
    throw new Error('Publication is still pending. Resume the same job to check its receipt.');
}
