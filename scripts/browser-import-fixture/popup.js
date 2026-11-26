const runtime = globalThis.browser ?? globalThis.chrome;
const openImport = document.getElementById('open-import');
const status = document.getElementById('status');

openImport.addEventListener('click', async () => {
    openImport.disabled = true;
    status.textContent = '';

    try {
        await runtime.tabs.create({ url: runtime.runtime.getURL('index.html') });
    } catch {
        status.textContent = 'Não foi possível abrir a aba de importação. Tente novamente.';
        openImport.disabled = false;
        return;
    }

    globalThis.close();
});
