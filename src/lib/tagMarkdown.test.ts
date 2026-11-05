import { describe, expect, it } from 'vitest';
import { parseTagMarkdown, serializeTagMarkdown } from './tagMarkdown';
import type { Tag } from './tags';

describe('tag Markdown outlines', () => {
    it('preserves hierarchy, sibling order, explicit colors, and inherited colors', () => {
        const tags: Tag[] = [
            {
                id: 'child',
                name: 'Child',
                parent_id: 'root',
                color: '#ff0000',
                order_index: 0,
                inherits_color: true
            },
            {
                id: 'last',
                name: 'Last',
                parent_id: null,
                color: '#abc',
                order_index: 100,
                inherits_color: false
            },
            {
                id: 'root',
                name: 'Root',
                parent_id: null,
                color: '#ff0000',
                order_index: 0,
                inherits_color: false
            },
            {
                id: 'custom',
                name: 'Custom',
                parent_id: 'root',
                color: '#ff0000',
                order_index: 100,
                inherits_color: false
            }
        ];
        expect(parseTagMarkdown(serializeTagMarkdown(tags))).toEqual([
            { name: 'Root', parentIndex: null, color: '#ff0000' },
            { name: 'Child', parentIndex: 0, color: null },
            { name: 'Custom', parentIndex: 0, color: '#ff0000' },
            { name: 'Last', parentIndex: null, color: '#abc' }
        ]);
    });

    it('round trips ambiguous names containing color metadata and quotes', () => {
        const tags: Tag[] = [
            { id: 'tag', name: 'A: color=#abc', parent_id: null, color: null, order_index: 0 }
        ];
        expect(parseTagMarkdown(serializeTagMarkdown(tags))[0].name).toBe(tags[0].name);
        tags[0].name = '"quoted"';
        expect(parseTagMarkdown(serializeTagMarkdown(tags))[0].name).toBe(tags[0].name);
    });

    it.each([
        '',
        '  - Missing root',
        '- Parent\n    - Skipped level',
        '- One\n - Odd indentation',
        '- Name\n- Name',
        '- Name: color=oops',
        '- Name: color=',
        '\t- Tab',
        '- ""',
        '- "Line\\nbreak"',
        'Plain text'
    ])('rejects malformed input: %j', markdown => {
        expect(() => parseTagMarkdown(markdown)).toThrow();
    });

    it('accepts CRLF, blank lines, nested siblings, and optional colors', () => {
        expect(
            parseTagMarkdown('- Parent\r\n\r\n  - Child\r\n  - Sibling: color=#abcd\r\n- Root')[2]
        ).toEqual({ name: 'Sibling', parentIndex: 0, color: '#abcd' });
    });

    it('rejects export of disconnected hierarchies instead of silently omitting tags', () => {
        expect(() =>
            serializeTagMarkdown([
                { id: 'tag', name: 'Lost', parent_id: 'missing', color: null, order_index: 0 }
            ])
        ).toThrow();
    });
});
