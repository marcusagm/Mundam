import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdir, open } from 'node:fs/promises';
import { dirname, join } from 'node:path';

/** Reads a regular local file without allocating beyond the protocol limit. */
export async function readBounded(path, limit) {
    const file = await open(path, 'r');
    try {
        const stat = await file.stat();
        if (!stat.isFile() || stat.size > limit)
            throw new Error('File is not regular or exceeds the size limit.');
        const bytes = Buffer.alloc(stat.size + 1);
        let length = 0;
        while (length < bytes.length) {
            const { bytesRead } = await file.read(bytes, length, bytes.length - length, length);
            if (!bytesRead) break;
            length += bytesRead;
        }
        if (length !== stat.size) throw new Error('File changed while reading.');
        return bytes.subarray(0, length);
    } finally {
        await file.close();
    }
}

/** Uses Windows DPAPI CurrentUser without putting credentials in process arguments. */
export async function protectProfile(bytes, decrypt = false) {
    if (process.platform !== 'win32' || !process.env.SystemRoot) {
        throw new Error('The reference client uses the Windows user profile.');
    }
    const method = decrypt ? 'Unprotect' : 'Protect';
    const command = `
        $ErrorActionPreference = 'Stop'
        Add-Type -AssemblyName System.Security
        $data = [Convert]::FromBase64String([Console]::In.ReadToEnd())
        $entropy = [Text.Encoding]::UTF8.GetBytes('Mundam external CLI profile v1')
        $output = [Security.Cryptography.ProtectedData]::${method}(
            $data, $entropy, [Security.Cryptography.DataProtectionScope]::CurrentUser)
        [Console]::Write([Convert]::ToBase64String($output))
    `;
    const exe = join(
        process.env.SystemRoot,
        'System32',
        'WindowsPowerShell',
        'v1.0',
        'powershell.exe'
    );
    const child = spawn(
        exe,
        [
            '-NoProfile',
            '-NonInteractive',
            '-EncodedCommand',
            Buffer.from(command, 'utf16le').toString('base64')
        ],
        {
            windowsHide: true,
            stdio: ['pipe', 'pipe', 'ignore']
        }
    );
    const closed = once(child, 'close');
    child.stdin.on('error', () => {});
    child.stdin.end(bytes.toString('base64'));
    const chunks = [];
    child.stdout.on('data', value => chunks.push(value));
    const [code] = await closed;
    if (code !== 0 || !chunks.length)
        throw new Error('The Windows user profile could not protect or open the credential.');
    return Buffer.from(Buffer.concat(chunks).toString('ascii'), 'base64');
}

/** Saves only the encrypted credential and public trust information. */
export async function saveProfile(path, profile) {
    const encrypted = await protectProfile(Buffer.from(JSON.stringify(profile)));
    await mkdir(dirname(path), { recursive: true });
    const file = await open(path, 'wx', 0o600);
    try {
        await file.writeFile(encrypted);
        await file.sync();
    } finally {
        await file.close();
    }
}

/** Opens a credential scoped to the current Windows user. */
export async function loadProfile(path) {
    const protectedBytes = await readBounded(path, 32 * 1024);
    return JSON.parse((await protectProfile(protectedBytes, true)).toString('utf8'));
}
