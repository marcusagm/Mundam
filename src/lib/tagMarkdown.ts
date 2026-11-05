import type { Tag } from './tags';

/** One tag in a preorder outline with an optional explicit color. */
export interface TagOutlineEntry {
    name: string;
    parentIndex: number | null;
    color: string | null;
}

/** Parses a Markdown tag list, rejecting malformed lines before import.
 * @param markdown - List using two spaces per level and optional color metadata.
 * @returns Ordered entries referencing their parent by index.
 * @throws Error with a line number when syntax or names are invalid.
 */
export function parseTagMarkdown(markdown: string): TagOutlineEntry[] {
    const entries: TagOutlineEntry[] = [];
    const ancestors: number[] = [];
    const names = new Set<string>();
    for (const [lineIndex, line] of markdown.split(/\r?\n/).entries()) {
        if (!line.trim()) continue;
        const match = /^( *)[-*+] (.+)$/.exec(line);
        if (!match || match[1].length % 2 !== 0) {
            throw new Error(`Line ${lineIndex + 1}: use a list item and two spaces per level.`);
        }
        const depth = match[1].length / 2;
        if (depth > ancestors.length) {
            throw new Error(`Line ${lineIndex + 1}: a parent level is missing.`);
        }
        const entry = parseTagLine(match[2], lineIndex + 1);
        if (names.has(entry.name)) {
            throw new Error(`Line ${lineIndex + 1}: duplicate tag "${entry.name}".`);
        }
        names.add(entry.name);
        entries.push({ ...entry, parentIndex: depth === 0 ? null : ancestors[depth - 1] });
        ancestors[depth] = entries.length - 1;
        ancestors.length = depth + 1;
    }
    if (!entries.length) throw new Error('Enter at least one tag.');
    return entries;
}

/** Reads optional color metadata while allowing JSON quoting for ambiguous names. */
function parseTagLine(content: string, lineNumber: number): Omit<TagOutlineEntry, 'parentIndex'> {
    const metadata = /^(.*):\s*color=(.*?)\s*$/.exec(content);
    const rawName = (metadata?.[1] ?? content).trim();
    const color = metadata?.[2] ?? 'inherit';
    if (color !== 'inherit' && !/^#(?:[\da-f]{3}|[\da-f]{4}|[\da-f]{6}|[\da-f]{8})$/i.test(color)) {
        throw new Error(`Line ${lineNumber}: use a hexadecimal color or color=inherit.`);
    }
    return { name: parseTagName(rawName, lineNumber), color: color === 'inherit' ? null : color };
}

/** Decodes quoted names and enforces the single-line display name contract. */
function parseTagName(rawName: string, lineNumber: number): string {
    let name: unknown = rawName;
    if (rawName.startsWith('"')) {
        try {
            name = JSON.parse(rawName);
        } catch {
            throw new Error(`Line ${lineNumber}: invalid quoted name.`);
        }
    }
    if (typeof name !== 'string' || !name.trim() || /[\r\n]/.test(name)) {
        throw new Error(`Line ${lineNumber}: enter a nonempty tag name on one line.`);
    }
    return name.trim();
}

/** Serializes tags in sibling order, retaining explicit colors and inheritance.
 * @param tags - Flat taxonomy with persisted hierarchy and effective colors.
 * @returns A Markdown outline that can be imported without losing color behavior.
 */
export function serializeTagMarkdown(tags: readonly Tag[]): string {
    const children = new Map<string | null, Tag[]>();
    for (const tag of tags) {
        const siblings = children.get(tag.parent_id) ?? [];
        siblings.push(tag);
        children.set(tag.parent_id, siblings);
    }
    const lines: string[] = [];
    const visited = new Set<string>();
    const appendChildren = (parentId: string | null, depth: number): void => {
        const siblings = children.get(parentId) ?? [];
        siblings.sort((first, second) => first.order_index - second.order_index);
        for (const tag of siblings) {
            if (visited.has(tag.id)) throw new Error('Cannot export a cyclic tag hierarchy.');
            visited.add(tag.id);
            const name =
                tag.name.startsWith('"') || /:\s*color=/.test(tag.name)
                    ? JSON.stringify(tag.name)
                    : tag.name;
            const color = tag.inherits_color || !tag.color ? 'inherit' : tag.color;
            lines.push(`${'  '.repeat(depth)}- ${name}: color=${color}`);
            appendChildren(tag.id, depth + 1);
        }
    };
    appendChildren(null, 0);
    if (visited.size !== tags.length) throw new Error('Cannot export an invalid tag hierarchy.');
    return lines.length ? `${lines.join('\n')}\n` : '';
}
