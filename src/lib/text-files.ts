import { invokeCommand } from './api';

/** Reads an authorized text file through the application IPC admission gate. */
export function readTextFile(path: string): Promise<string> {
    return invokeCommand<string>('read_text_file', { path });
}

/** Awaits an authorized export before its command releases shutdown admission. */
export function writeTextFile(path: string, contents: string): Promise<void> {
    return invokeCommand<void>('write_text_file', { path, contents });
}
