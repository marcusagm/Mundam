const runtime = globalThis.browser ?? globalThis.chrome;
const controls = Object.fromEntries(
    ['folder', 'file', 'connect', 'import', 'resume', 'status'].map(id => [
        id,
        document.getElementById(id)
    ])
);
const storageKey = 'mundamImportBridgeJob';
const partBytes = 256 * 1024;
const maxFileBytes = 16 * 1024 * 1024;
const itemId = 'image';
let port;
let waiting;
let busy = false;

function failPending(error) {
    if (!waiting) return;
    const request = waiting;
    waiting = undefined;
    clearTimeout(request.timeout);
    request.reject(error);
}

function connect() {
    const connection = runtime.runtime.connectNative('com.mundam.import_bridge');
    connection.onMessage.addListener(response => {
        if (connection !== port || !waiting) return;
        if (response?.version !== 1 || response.id !== waiting.id) {
            return failPending(new Error('Resposta inesperada da ponte.'));
        }
        const request = waiting;
        waiting = undefined;
        clearTimeout(request.timeout);
        if (response.error) request.reject(new Error(`Ponte: ${response.error}`));
        else request.resolve(response.data);
    });
    connection.onDisconnect.addListener(() => {
        // Chromium requires lastError to be read in the disconnect callback.
        void runtime.runtime.lastError;
        if (connection !== port) return;
        port = undefined;
        failPending(new Error('Ponte desconectada. Verifique o registro do host e tente retomar.'));
    });
    return connection;
}

function request(op, fields = {}) {
    port ??= connect();
    if (waiting) throw new Error('A ponte ainda tem uma operação em andamento.');
    const id = crypto.randomUUID();
    return new Promise((resolve, reject) => {
        const timeout = setTimeout(() => {
            failPending(new Error('A ponte não respondeu. Retome o lote salvo.'));
            port?.disconnect();
            port = undefined;
        }, 25_000);
        waiting = { id, resolve, reject, timeout };
        try {
            port.postMessage({ version: 1, id, op, ...fields });
        } catch {
            port?.disconnect();
            port = undefined;
            failPending(new Error('A ponte não está disponível.'));
        }
    });
}

function progress(text) {
    controls.status.textContent = text;
}

function toggle() {
    controls.connect.disabled = busy;
    controls.resume.disabled = busy;
    controls.import.disabled = busy || !controls.folder.value;
    controls.folder.disabled = busy || !controls.folder.options.length;
    controls.file.disabled = busy;
}

async function run(action) {
    if (busy) return;
    busy = true;
    toggle();
    try {
        await action();
    } catch (error) {
        progress(error.message);
    } finally {
        busy = false;
        toggle();
    }
}

async function listFolders() {
    const capabilities = await request('capabilities');
    if (!capabilities.publicationAvailable || !capabilities.mediaTypes.includes('image/png')) {
        throw new Error('O Mundam precisa estar com a publicação de PNG habilitada.');
    }
    const { items } = await request('folders');
    controls.folder.replaceChildren(...items.map(folder => new Option(folder.name, folder.id)));
    progress(
        items.length ? 'Selecione um PNG ou retome o lote salvo.' : 'Nenhuma raiz foi concedida.'
    );
}

async function selectedFile() {
    const [file] = controls.file.files;
    if (!file || file.size < 8 || file.size > maxFileBytes || !/\.png$/i.test(file.name)) {
        throw new Error('Selecione um PNG de até 16 MiB.');
    }
    const bytes = new Uint8Array(await file.arrayBuffer());
    const sha256 = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes)), value =>
        value.toString(16).padStart(2, '0')
    ).join('');
    return { file, bytes, sha256 };
}

function base64(bytes) {
    let binary = '';
    for (let position = 0; position < bytes.length; position += 8192) {
        binary += String.fromCharCode(...bytes.subarray(position, position + 8192));
    }
    return btoa(binary);
}

async function publish(job, selected) {
    if (Date.now() >= job.createdAt + 24 * 60 * 60 * 1000) {
        throw new Error('O lote salvo passou do prazo de 24 horas.');
    }
    const admission = await request('admit', { key: job.key, manifest: job.manifest });
    if (admission.partSizeBytes !== partBytes) throw new Error('Tamanho de bloco inesperado.');
    const batchId = admission.batchId;
    let receipt = await request('receipt', { batchId });
    if (receipt.state !== 'committed') {
        const declared = job.manifest.items[0];
        const confirmed = new Set();
        for (let page = 0; page < 16; page++) {
            for (const part of receipt.confirmedParts) confirmed.add(part.partNumber);
            if (!receipt.nextCursor) break;
            if (page === 15) throw new Error('O recibo excedeu o limite de páginas.');
            receipt = await request('receipt', { batchId, cursor: receipt.nextCursor });
        }
        if (
            ['failed', 'cancelled'].includes(receipt.items[0].state) &&
            !receipt.items[0].error?.retryable
        ) {
            throw new Error(receipt.items[0].error?.code ?? 'O lote foi cancelado.');
        }
        const count = Math.ceil(declared.byteLength / partBytes);
        if (confirmed.size < count) {
            selected ??= await selectedFile();
            if (selected.sha256 !== job.sha256 || selected.bytes.length !== declared.byteLength) {
                throw new Error('Selecione o PNG original para reenviar os blocos ausentes.');
            }
            for (let part = 0; part < count; part++) {
                if (confirmed.has(part)) continue;
                await request('part', {
                    batchId,
                    itemId,
                    part,
                    base64: base64(
                        selected.bytes.subarray(part * partBytes, (part + 1) * partBytes)
                    )
                });
                progress(`Bloco ${part + 1} de ${count} confirmado.`);
            }
        }
        await request('commit', { batchId });
        const deadline = Date.now() + 30_000;
        do {
            await new Promise(resolve => setTimeout(resolve, 300));
            receipt = await request('receipt', { batchId });
            if (['failed', 'cancelled'].includes(receipt.state)) {
                throw new Error(receipt.items[0].error?.code ?? 'O lote foi cancelado.');
            }
        } while (receipt.state !== 'committed' && Date.now() < deadline);
    }
    if (receipt.state !== 'committed')
        throw new Error('Publicação pendente. Consulte pelo botão de retomada.');
    const { assetId, fileName } = receipt.items[0];
    progress(`Publicado: ${fileName}\nAsset: ${assetId}`);
}

controls.connect.addEventListener('click', () => void run(listFolders));
controls.import.addEventListener(
    'click',
    () =>
        void run(async () => {
            const selected = await selectedFile();
            const job = {
                key: crypto.randomUUID(),
                createdAt: Date.now(),
                sha256: selected.sha256,
                manifest: {
                    folderId: controls.folder.value,
                    items: [
                        { itemId, fileName: selected.file.name, byteLength: selected.bytes.length }
                    ]
                }
            };
            await runtime.storage.local.set({ [storageKey]: job });
            await publish(job, selected);
        })
);
controls.resume.addEventListener(
    'click',
    () =>
        void run(async () => {
            const { [storageKey]: job } = await runtime.storage.local.get(storageKey);
            if (!job) throw new Error('Nenhum lote foi salvo nesta extensão.');
            await publish(job);
        })
);
void run(listFolders);
