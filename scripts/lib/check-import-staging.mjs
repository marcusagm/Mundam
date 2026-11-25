import { randomUUID } from 'node:crypto';
import { digest, exchange } from './import-transport-probe.mjs';

/** Exercises bounded staging with disposable bytes and releases the reservation. */
export async function checkImportStaging(server, identity, token, folderId) {
    const send = async (method, route, expected, options = {}) => {
        const response = await exchange(server, identity, method, route, { token, ...options });
        if (response.status !== expected) {
            const code = response.data?.error?.code;
            throw new Error(
                `Staging check expected HTTP ${expected}, received ${response.status}${code ? ' (' + code + ')' : ''}.`
            );
        }
        return response.data;
    };
    const reservation = await send('POST', '/v1/imports', 201, {
        headers: { 'content-type': 'application/json', 'idempotency-key': randomUUID() },
        body: JSON.stringify({
            folderId,
            items: [{ itemId: 'sample', fileName: 'staging-check.bin', byteLength: 256 * 1024 + 3 }]
        })
    });
    const receipt = reservation.receiptPath;
    try {
        const sendPart = (number, body) =>
            send('PUT', `${receipt}/items/sample/parts/${number}`, 204, {
                headers: {
                    'content-type': 'application/octet-stream',
                    'x-part-sha256': digest(body)
                },
                body
            });
        await sendPart(1, Buffer.from('end'));
        const partial = await send('GET', receipt, 200);
        if (
            partial.state !== 'receiving' ||
            partial.confirmedParts.length !== 1 ||
            partial.confirmedParts[0].partNumber !== 1
        ) {
            throw new Error('Unexpected receipt for the partial upload.');
        }
        console.log('Partial receipt confirms part 1.');
        await sendPart(0, Buffer.alloc(reservation.partSizeBytes, 0x61));
        const complete = await send('GET', receipt, 200);
        if (complete.state !== 'ready') throw new Error('Expected the staged file to be ready.');
        console.log('Staging complete. Receipt is ready.');
    } finally {
        const cancelled = await send('DELETE', receipt, 202);
        if (cancelled.state !== 'cancelled' || cancelled.confirmedParts.length !== 0) {
            throw new Error('Cancellation did not clear the staged parts.');
        }
        console.log('Staging cancelled and cleared.');
    }
}
