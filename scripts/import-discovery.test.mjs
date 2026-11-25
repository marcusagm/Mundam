import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { mkdtemp, rm, rmdir, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { readDiscovery } from './lib/import-discovery.mjs';

test('discovers a changed port using only a previously trusted local identity', async context => {
    const directory = await mkdtemp(join(tmpdir(), 'mundam-discovery-'));
    const path = join(directory, 'import-endpoint.json');
    context.after(async () => {
        await rm(path, { force: true });
        await rmdir(directory);
    });
    const pin = 'a'.repeat(64);
    const first = {
        version: 1,
        instanceId: randomUUID(),
        url: 'https://127.0.0.1:50001',
        publicKeySha256: pin
    };
    assert.equal(await readDiscovery(path, pin), null);
    await writeFile(path, JSON.stringify(first));
    assert.deepEqual(await readDiscovery(path, pin), { port: 50001, instanceId: first.instanceId });
    const second = { ...first, url: 'https://127.0.0.1:50002', instanceId: randomUUID() };
    await writeFile(path, JSON.stringify(second));
    assert.deepEqual(await readDiscovery(path, pin), {
        port: 50002,
        instanceId: second.instanceId
    });
    for (const document of [
        { ...second, url: 'https://example.org:50002' },
        { ...second, url: 'http://127.0.0.1:50002' },
        { ...second, publicKeySha256: 'b'.repeat(64) },
        { ...second, version: 2 },
        { ...second, instanceId: '' },
        { ...second, url: 'https://127.0.0.1:70000' },
        { ...second, url: 'https://127.0.0.1:50002\n' }
    ]) {
        await writeFile(path, JSON.stringify(document));
        assert.equal(await readDiscovery(path, pin), null);
    }
    await writeFile(path, '{');
    assert.equal(await readDiscovery(path, pin), null);
    await writeFile(path, JSON.stringify({ ...second, ignored: 'x'.repeat(4096) }));
    assert.equal(await readDiscovery(path, pin), null);
    await rm(path);
    assert.equal(await readDiscovery(path, pin), null);
});
