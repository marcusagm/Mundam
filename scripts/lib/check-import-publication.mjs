import { randomUUID } from 'node:crypto';
import { setTimeout as pause } from 'node:timers/promises';
import { digest, exchange } from './import-transport-probe.mjs';

const sampleImage = Buffer.from(
    'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGMwn/jhPwAFcwK4lay5QwAAAABJRU5ErkJggg==',
    'base64'
);

/** Publishes one diagnostic PNG into the explicitly granted test folder. */
export async function checkImportPublication(server, identity, token, folderId) {
    const send = async (method, route, expected, options = {}) => {
        const response = await exchange(server, identity, method, route, { token, ...options });
        if (response.status !== expected) {
            throw new Error(
                `Publication expected HTTP ${expected}, received ${response.status} (${response.data?.error?.code ?? 'unknown'}).`
            );
        }
        return response.data;
    };
    const fileName = `mundam-publication-check-${randomUUID().slice(0, 8)}.png`;
    const sha256 = digest(sampleImage);
    const admission = await send('POST', '/v1/imports', 201, {
        headers: { 'content-type': 'application/json', 'idempotency-key': randomUUID() },
        body: JSON.stringify({
            folderId,
            items: [{ itemId: 'sample', fileName, byteLength: sampleImage.length }]
        })
    });
    const receiptPath = admission.receiptPath;
    await send('PUT', `${receiptPath}/items/sample/parts/0`, 204, {
        headers: { 'content-type': 'application/octet-stream', 'x-part-sha256': sha256 },
        body: sampleImage
    });
    await send('POST', `${receiptPath}/commit`, 202);
    console.log('Publication accepted. Waiting for the receipt.');
    const deadline = Date.now() + 30_000;
    while (Date.now() < deadline) {
        const receipt = await send('GET', receiptPath, 200);
        const item = receipt.items[0];
        if (item.state === 'failed') {
            throw new Error(`Publication failed (${item.error.code}).`);
        }
        if (receipt.state === 'committed' && item.assetId && item.fileName) {
            const replay = await send('POST', `${receiptPath}/commit`, 202);
            if (replay.state !== 'committed' || replay.items[0]?.assetId !== item.assetId) {
                throw new Error('Retry did not return the original publication receipt.');
            }
            console.log('Published PNG:', item.fileName);
            console.log('Asset ID:', item.assetId);
            console.log('Expected SHA-256:', sha256);
            console.log('Retry returned the original asset.');
            return;
        }
        await pause(300);
    }
    throw new Error('No committed receipt within 30 seconds.');
}
