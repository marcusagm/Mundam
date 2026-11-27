import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { before, test } from 'node:test';
import { transpileBytes } from '@bytecodealliance/jco-transpile';
import { componentNew, componentWit } from '@bytecodealliance/jco-transpile/wasm-tools';

const previewInterface = 'mundam:worker/image-preview';
const expectedImport = `${previewInterface}@0.1.0`;
const encoder = new TextEncoder();
const thumbnail = encoder.encode('RIFF....WEBP');
const expectedSignature = encoder.encode('RIFFWEBP');
const fixtureName = 'preview-check';
const coreWasm = new URL(
    './target/wasm32-unknown-unknown/release/mundam_wit_preview_check.wasm',
    import.meta.url
);

let componentContract;
let instantiate;
let coreModules;

before(async () => {
    const component = await componentNew(await readFile(coreWasm), []);
    componentContract = await componentWit(component);
    await writeFile(new URL('./preview-check.component.wasm', coreWasm), component);
    const { files } = await transpileBytes(component, {
        name: fixtureName,
        instantiation: 'sync',
        noComponentErrorWrapping: false,
        wasiShim: false
    });
    coreModules = new Map(
        Object.entries(files)
            .filter(([name]) => name.endsWith('.wasm'))
            .map(([name, bytes]) => [name, new WebAssembly.Module(bytes)])
    );
    const script = files[`${fixtureName}.js`];
    assert.ok(script, 'expected generated component bindings');
    const generated = new URL('./preview-check.generated.mjs', coreWasm);
    await writeFile(generated, script);
    ({ instantiate } = await import(generated.href));
});

function loadCoreModule(name) {
    assert.ok(coreModules.has(name), `unexpected module ${name}`);
    return coreModules.get(name);
}

function witError(payload) {
    return Object.assign(new Error(payload), { payload });
}

function expectWitError(guest, payload) {
    assert.throws(
        () => guest.readPreviewSignature(),
        error => guest._util.isComponentError(error) && error.payload === payload
    );
}

function loadFixture({ openError, secondReadError, bytes = thumbnail, shortRead = false } = {}) {
    const calls = [];
    class HostPreview {
        byteCount() {
            calls.push(['byte-count']);
            return bytes.length;
        }

        read(offset, length) {
            calls.push(['read', offset, length]);
            if (secondReadError && offset === 8) {
                throw witError(secondReadError);
            }
            return bytes.slice(offset, offset + length - Number(shortRead));
        }

        [Symbol.dispose]() {
            calls.push(['drop']);
        }
    }

    const guest = instantiate(loadCoreModule, {
        [previewInterface]: {
            Preview: HostPreview,
            openSelectedPreview() {
                calls.push(['open']);
                if (openError) {
                    throw witError(openError);
                }
                return new HostPreview();
            }
        }
    });
    return { guest, calls };
}

test('the component imports only the versioned preview capability', () => {
    const interfaces = [...componentContract.matchAll(/^\s*import ([^;]+);$/gm)].map(
        match => match[1]
    );
    assert.deepEqual(interfaces, [expectedImport]);
    assert.match(componentContract, /export read-preview-signature: func\(/);
    assert.throws(() => instantiate(loadCoreModule, {}));
});

test('an owned preview supplies both ranges and is released after success', () => {
    const { guest, calls } = loadFixture();
    assert.deepEqual(guest.readPreviewSignature(), expectedSignature);
    assert.deepEqual(calls, [['open'], ['byte-count'], ['read', 0, 4], ['read', 8, 4], ['drop']]);
});

test('a denied preview is reported as unavailable before any read', () => {
    const { guest, calls } = loadFixture({ openError: 'unavailable' });
    expectWitError(guest, 'unavailable');
    assert.deepEqual(calls, [['open']]);
});

test('a mid-read budget error propagates and releases the resource', () => {
    const { guest, calls } = loadFixture({ secondReadError: 'budget-exhausted' });
    expectWitError(guest, 'budget-exhausted');
    assert.deepEqual(calls, [['open'], ['byte-count'], ['read', 0, 4], ['read', 8, 4], ['drop']]);
});

test('a short preview becomes a typed range error without requesting bytes', () => {
    const { guest, calls } = loadFixture({ bytes: thumbnail.subarray(0, 8) });
    expectWitError(guest, 'invalid-range');
    assert.deepEqual(calls, [['open'], ['byte-count'], ['drop']]);
});

test('an incomplete host response cannot be returned as a signature', () => {
    const { guest, calls } = loadFixture({ shortRead: true });
    expectWitError(guest, 'unavailable');
    assert.deepEqual(calls, [['open'], ['byte-count'], ['read', 0, 4], ['drop']]);
});
