import { once } from 'node:events';

export const maxFrameBytes = 384 * 1024;

/** Decodes the bounded, length-prefixed UTF-8 frames used by the Windows host. */
export async function* readNativeFrames(input) {
    const header = Buffer.alloc(4);
    const decoder = new TextDecoder('utf-8', { fatal: true });
    let target = header;
    let offset = 0;
    for await (const chunk of input) {
        let cursor = 0;
        while (cursor < chunk.length) {
            const count = Math.min(chunk.length - cursor, target.length - offset);
            chunk.copy(target, offset, cursor, cursor + count);
            cursor += count;
            offset += count;
            if (offset < target.length) continue;
            if (target === header) {
                const length = header.readUInt32LE();
                if (length < 1 || length > maxFrameBytes)
                    throw new Error('Invalid native frame length.');
                target = Buffer.alloc(length);
            } else {
                const value = JSON.parse(decoder.decode(target));
                target = header;
                yield value;
            }
            offset = 0;
        }
    }
    if (target !== header || offset) throw new Error('Truncated native frame.');
}

/** Emits one response and respects stdout backpressure. */
export async function writeNativeFrame(output, value) {
    const body = Buffer.from(JSON.stringify(value));
    if (body.length > maxFrameBytes) throw new Error('Native response exceeds the size limit.');
    const frame = Buffer.alloc(4 + body.length);
    frame.writeUInt32LE(body.length);
    body.copy(frame, 4);
    if (!output.write(frame)) await once(output, 'drain');
}
