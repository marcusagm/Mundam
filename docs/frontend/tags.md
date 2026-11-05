# Tag outlines and color inheritance

Open **Import / Export Tags** using the list button beside **Create Tag** in the
Tags sidebar. Paste a Markdown list, using two spaces per hierarchy level. Sibling
order follows the list, independently of alphabetical order.

```markdown
- Art: color=#e57373
  - Painting: color=inherit
  - Drawing: color=#64b5f6
- Design
```

Colors accept hexadecimal RGB/RGBA forms (`#abc`, `#abcd`, `#aabbcc`,
`#aabbccdd`). Omit the color or write `color=inherit` to follow the parent.
Root tags without an explicit color use the default icon appearance. Lists also
accept `*` or `+` bullets and blank lines. Names containing the reserved
`: color=` suffix or beginning with a quote are JSON-quoted during export:

```markdown
- "A: color=#abc": color=inherit
```

**Keep existing tags** is selected by default. Matching names under the same
parent reuse their identifiers and asset associations, while taking the imported
color behavior and order. Tags outside the list remain first in their existing
sibling order; imported siblings follow in list order. Since names are globally
unique, an existing name under a different parent rejects the entire import.
Empty outlines, duplicate names, malformed colors and skipped indentation levels
are rejected before import.

Deselecting **Keep existing tags** shows the destructive warning and changes the
action to **Replace All Tags**. Replacement deletes the entire existing taxonomy
and every asset-to-tag association. The complete import runs in a single ledger
transaction: any error rolls back tag creation, deletion and association changes.
After a successful replacement, the sidebar's deleted tag selection is cleared.

**Export Existing Tags** saves the persisted taxonomy to a `.md` file. It exports
hierarchy, sibling order, explicit colors and `color=inherit`, so reimporting
preserves future color behavior as well as the current appearance. The editor's
unsaved contents do not affect export.

New child tags inherit by default. Picking a color in the context menu makes that
tag's color explicit, even if it equals its parent's current color. **Use Parent
Color** restores inheritance; a check icon indicates that it is active. Changing
a parent color updates inherited descendants, stopping at custom colors; children
below a custom color continue to inherit from that custom parent. Reparenting also
recomputes inherited colors and rejects cycles. Root inherited tags return to the
default appearance when detached from their parent.

The migration preserves every pre-existing explicit color as custom and enables
inheritance for tags that previously had no color. Effective colors are persisted
alongside the inheritance flag, so asset tag queries and sidebar tags agree.

Implementation follows delivery → application handler → ledger → transactional
database handlers for imports. Pure outline validation lives in
`core/ledger/tag_validation.rs`. Import and color mutations emit ledger events
after commit, and frontend actions reconcile tags and notify dependent asset and
statistics views.
