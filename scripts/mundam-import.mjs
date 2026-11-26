import { stat } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { parseArgs } from 'node:util';
import { loadProfile, saveProfile } from './lib/import-client-storage.mjs';
import { windowsDiscoveryPath } from './lib/import-discovery.mjs';
import { loadJob, prepareJob, runJob, saveJob } from './lib/import-job.mjs';
import {
    createSession,
    expectStatus,
    pairImporter,
    readInvitation
} from './lib/import-session.mjs';

const usage = `Usage
  mundam-import pair [--name LABEL] [--profile PATH]     reads the invitation from stdin
  mundam-import folders [--parent ID] [--cursor CURSOR] [--profile PATH]
  mundam-import tags [--query TEXT] [--cursor CURSOR] [--profile PATH]
  mundam-import import --job PATH [--folder ID] [--tag ID]... [--source URL] [--profile PATH] FILE.png ...
  mundam-import resume --job PATH [--profile PATH]`;
const options = {
    pair: { name: { type: 'string', default: 'Mundam reference client' } },
    folders: { parent: { type: 'string' }, cursor: { type: 'string' } },
    tags: { query: { type: 'string' }, cursor: { type: 'string' } },
    import: {
        job: { type: 'string' },
        folder: { type: 'string' },
        tag: { type: 'string', multiple: true, default: [] },
        source: { type: 'string' }
    },
    resume: { job: { type: 'string' } }
};

async function pair(profilePath, clientName) {
    const existing = await stat(profilePath).catch(error => {
        if (error.code === 'ENOENT') return null;
        throw error;
    });
    if (existing)
        throw new Error('A profile already exists at that path. Choose a new --profile path.');
    const invitation = await readInvitation(process.stdin);
    const scopes = ['folders.read', 'tags.read', 'assets.import', 'tags.assignOnImport'];
    const { profile } = await pairImporter(invitation, clientName, scopes, () => {
        console.error('Review the requested scopes and roots in Mundam, then approve.');
    });
    await saveProfile(profilePath, profile);
    console.error('Pairing saved for the current Windows user.');
}

async function list(command, values, send) {
    const query = new URLSearchParams();
    if (values.parent) query.set('parentId', values.parent);
    if (values.query) query.set('query', values.query);
    if (values.cursor) query.set('cursor', values.cursor);
    const path = `/v1/${command}${query.size ? `?${query}` : ''}`;
    const page = expectStatus(await send('GET', path), 200);
    console.log(JSON.stringify(page, null, 2));
}

async function main() {
    const [command, ...args] = process.argv.slice(2);
    if (!command || command === '--help' || args.includes('--help')) {
        console.log(usage);
        return;
    }
    if (!Object.hasOwn(options, command)) throw new Error('Unknown command. Run with --help.');
    const { values, positionals } = parseArgs({
        args,
        options: { ...options[command], profile: { type: 'string' } },
        allowPositionals: command === 'import'
    });
    const discovery = windowsDiscoveryPath();
    const profilePath = values.profile
        ? resolve(values.profile)
        : join(dirname(discovery), 'import-client.v1.dpapi');
    if (command === 'pair') {
        await pair(profilePath, values.name);
        return;
    }
    const profile = await loadProfile(profilePath);
    const send = createSession(profile, discovery);
    if (command === 'tags' || command === 'folders') {
        await list(command, values, send);
        return;
    }
    if (!values.job) throw new Error('Provide --job with a local job file path.');
    const jobPath = resolve(values.job);
    let job;
    if (command === 'import') {
        const capabilities = expectStatus(await send('GET', '/v1/capabilities'), 200);
        if (
            !capabilities?.publicationAvailable ||
            !capabilities.mediaTypes?.includes('image/png') ||
            !capabilities.grants?.includes('assets.import')
        ) {
            throw new Error('The paired client needs PNG publication and the assets.import grant.');
        }
        if (values.tag.length && !capabilities.grants.includes('tags.assignOnImport')) {
            throw new Error('The paired client needs tags.assignOnImport to assign tags.');
        }
        job = await prepareJob({
            paths: positionals,
            folderId: values.folder,
            tagIds: values.tag,
            sourceUrl: values.source,
            profile
        });
        await saveJob(jobPath, job);
        console.error(`Job saved: ${jobPath}`);
    } else {
        job = await loadJob(jobPath, profile);
    }
    const receipt = await runJob(job, send, (name, part, total) => {
        console.error(`Confirmed ${name} part ${part}/${total}`);
    });
    console.log(JSON.stringify(receipt, null, 2));
    if (receipt.state !== 'committed') process.exitCode = 1;
}

try {
    await main();
} catch (error) {
    console.error(error instanceof Error ? error.message : 'Mundam import failed.');
    process.exitCode = 1;
}
