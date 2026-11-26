import { digest } from './import-http.mjs';
import { readNativeFrames, writeNativeFrame } from './native-message-frames.mjs';

const maxPartBytes = 256 * 1024;
const uuid = /^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/;
const itemPattern = /^[A-Za-z0-9_-]{1,128}$/;
const jsonHeaders = { 'content-type': 'application/json' };
const string = (value, maximum) =>
    typeof value === 'string' && value.length > 0 && value.length <= maximum;
const batch = value => {
    if (!string(value, 36) || !uuid.test(value)) invalid();
    return value;
};

class BridgeError extends Error {
    constructor(code) {
        super(code);
        this.code = code;
    }
}

function invalid() {
    throw new BridgeError('INVALID_MESSAGE');
}

function shape(value, keys) {
    if (
        !value ||
        typeof value !== 'object' ||
        Array.isArray(value) ||
        Object.keys(value).some(key => !keys.includes(key))
    )
        invalid();
}

function queryPath(path, fields) {
    const query = new URLSearchParams();
    for (const [key, value] of Object.entries(fields)) {
        if (value === undefined) continue;
        if (!string(value, key === 'cursor' ? 2048 : 256) || /[\u0000-\u001f]/.test(value))
            invalid();
        query.set(key, value);
    }
    return query.size ? `${path}?${query}` : path;
}

function manifest(value) {
    shape(value, ['folderId', 'tagIds', 'items']);
    const tags = ids => {
        if (
            ids !== undefined &&
            (!Array.isArray(ids) || ids.length > 32 || ids.some(id => !string(id, 128)))
        )
            invalid();
    };
    if (value.folderId !== undefined && !string(value.folderId, 128)) invalid();
    tags(value.tagIds);
    if (!Array.isArray(value.items) || !value.items.length || value.items.length > 8) invalid();
    let total = 0;
    for (const item of value.items) {
        shape(item, ['itemId', 'fileName', 'byteLength', 'sourceUrl', 'tagIds']);
        if (
            !string(item.itemId, 128) ||
            !itemPattern.test(item.itemId) ||
            !string(item.fileName, 240) ||
            !Number.isSafeInteger(item.byteLength) ||
            item.byteLength < 1 ||
            item.byteLength > 16 * 1024 * 1024
        )
            invalid();
        if (item.sourceUrl !== undefined && !string(item.sourceUrl, 2048)) invalid();
        tags(item.tagIds);
        total += item.byteLength;
    }
    if (total > 32 * 1024 * 1024) invalid();
    return value;
}

function command(message) {
    const names = ['version', 'id', 'op'];
    if (message?.version !== 1 || !string(message.id, 64) || !/^[A-Za-z0-9_-]+$/.test(message.id))
        invalid();
    switch (message.op) {
        case 'capabilities':
            shape(message, names);
            return ['GET', '/v1/capabilities'];
        case 'folders':
        case 'tags': {
            const key = message.op === 'folders' ? 'parentId' : 'query';
            shape(message, [...names, key, 'cursor']);
            return [
                'GET',
                queryPath(`/v1/${message.op}`, { [key]: message[key], cursor: message.cursor })
            ];
        }
        case 'admit':
            shape(message, [...names, 'key', 'manifest']);
            if (!string(message.key, 36) || !uuid.test(message.key)) invalid();
            return [
                'POST',
                '/v1/imports',
                {
                    headers: { ...jsonHeaders, 'idempotency-key': message.key },
                    body: JSON.stringify(manifest(message.manifest))
                }
            ];
        case 'part': {
            shape(message, [...names, 'batchId', 'itemId', 'part', 'base64']);
            const batchId = batch(message.batchId);
            if (
                !string(message.itemId, 128) ||
                !itemPattern.test(message.itemId) ||
                !Number.isSafeInteger(message.part) ||
                message.part < 0 ||
                message.part >= 64 ||
                !string(message.base64, Math.ceil(maxPartBytes / 3) * 4)
            )
                invalid();
            const body = Buffer.from(message.base64, 'base64');
            if (
                !body.length ||
                body.length > maxPartBytes ||
                body.toString('base64') !== message.base64
            )
                invalid();
            return [
                'PUT',
                `/v1/imports/${batchId}/items/${message.itemId}/parts/${message.part}`,
                {
                    headers: {
                        'content-type': 'application/octet-stream',
                        'x-part-sha256': digest(body)
                    },
                    body
                }
            ];
        }
        case 'receipt':
            shape(message, [...names, 'batchId', 'cursor']);
            return [
                'GET',
                queryPath(`/v1/imports/${batch(message.batchId)}`, { cursor: message.cursor })
            ];
        case 'commit':
        case 'cancel':
            shape(message, [...names, 'batchId']);
            return message.op === 'commit'
                ? ['POST', `/v1/imports/${batch(message.batchId)}/commit`]
                : ['DELETE', `/v1/imports/${batch(message.batchId)}`];
        default:
            invalid();
    }
}

/** Runs only the typed import protocol. Browser frames never supply HTTP headers or filesystem paths. */
export async function acceptNativeMessage(message, send) {
    const id = string(message?.id, 64) && /^[A-Za-z0-9_-]+$/.test(message.id) ? message.id : null;
    let request;
    try {
        request = command(message);
    } catch {
        return { version: 1, id, error: 'INVALID_MESSAGE' };
    }
    try {
        const { status, data } = await send(...request);
        if (status >= 200 && status < 300) return { version: 1, id, status, data };
        const code = data?.error?.code;
        return {
            version: 1,
            id,
            status,
            error: string(code, 64) && /^[A-Z_]+$/.test(code) ? code : 'REQUEST_FAILED'
        };
    } catch {
        return { version: 1, id, error: 'MUNDAM_UNAVAILABLE' };
    }
}

/** Serializes one operation and one response at a time for a native port. */
export async function runNativeBridge(input, output, send) {
    for await (const message of readNativeFrames(input)) {
        await writeNativeFrame(output, await acceptNativeMessage(message, send));
    }
}
