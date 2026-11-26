import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { runInNewContext } from 'node:vm';

const fixture = new URL('./browser-import-fixture/', import.meta.url);
const readFixture = name => readFile(new URL(name, fixture), 'utf8');

test('browser popup opens the import form in a tab with the existing permissions', async () => {
    const manifest = JSON.parse(await readFixture('manifest.json'));
    assert.equal(manifest.action.default_popup, 'popup.html');
    assert.deepEqual(manifest.permissions, ['nativeMessaging', 'storage']);
    assert.doesNotMatch(await readFixture(manifest.action.default_popup), /type="file"/);
    assert.match(await readFixture('index.html'), /type="file"/);
    const launcher = await readFixture('popup.js');

    for (const [runtime, origin] of [
        ['browser', 'moz-extension://fixture'],
        ['chrome', 'chrome-extension://fixture']
    ]) {
        const events = [];
        let click;
        let fail = true;
        const button = {
            disabled: false,
            addEventListener(event, handler) {
                assert.equal(event, 'click');
                click = handler;
            }
        };
        const status = { textContent: '' };

        runInNewContext(launcher, {
            [runtime]: {
                runtime: { getURL: path => `${origin}/${path}` },
                tabs: {
                    async create({ url }) {
                        assert.equal(button.disabled, true);
                        events.push(url);
                        if (fail) throw new Error('Could not open tab');
                    }
                }
            },
            document: { getElementById: id => (id === 'open-import' ? button : status) },
            close: () => events.push('closed')
        });

        await click();
        assert.deepEqual(events, [`${origin}/index.html`]);
        assert.equal(button.disabled, false);
        assert.match(status.textContent, /Tente novamente/);

        fail = false;
        await click();
        assert.deepEqual(events, [`${origin}/index.html`, `${origin}/index.html`, 'closed']);
        assert.equal(status.textContent, '');
    }
});
