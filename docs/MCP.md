# Create and edit DOCX files from an MCP client

The local `rocx-mcp` server lets a client create a Word document or edit a
copy of an existing one. From a repository checkout, install it with:

```sh
cargo install --locked --path crates/rocx-mcp
```

Configure a **stdio** server in your client. For clients that use the
`mcpServers` configuration key:

```json
{
  "mcpServers": {
    "rocx": { "command": "rocx-mcp", "args": [] }
  }
}
```

If the client cannot find the executable, use its absolute installed path as
`command`. The client launches the process and exchanges MCP messages on stdin
and stdout. Document and output paths refer to the server's filesystem;
use absolute paths if the client controls the working directory.

## Create a document

After initialization, use `tools/list` to discover the tools and their input
schemas. The following are tool arguments; substitute the actual handles
returned by the server for the placeholders.

1. Call `create_document` with `{"paragraphs":["Quarterly report","Draft"]}`.
   It returns a `document` and an initial `preview`. Pass `{}` for a blank
   document with one editable paragraph. Neither call writes a file.
2. To save the document as created, call `save_copy` with
   `{"document":"<document>","preview":"<initial preview>","output":"report.docx"}`.
3. To edit before saving, call `inspect_document` with `{"document":"<document>"}`.
   Find the paragraph containing “Quarterly report” and copy its `anchor`.
4. Call `preview_edits`:

   ```json
   {
     "document": "<document>",
     "edits": [
       {"kind":"alignment","anchor":"<paragraph anchor>","alignment":"center"}
     ]
   }
   ```

5. Pass the **new preview** to `save_copy` with a fresh output path, then call
   `close_document` with `{"document":"<document>"}`.

## Edit an existing file

Start with `open_document` and `{"path":"input.docx"}`, then inspect, preview,
and save as above. The supported edits are paragraph alignment, paragraph or
table insertion after a paragraph, direct run text replacement or removal,
replacement/deletion of a text range or insertion at a text position,
physical table-cell vertical alignment, direct resource edits, and image
insertion in a header or footer. For a story image, specify `shared` to edit
its shared part or `section-local` to limit the change to the selected section.

To override archive read budgets for one open, pass, for example,
`{"path":"input.docx","limits":{"max_parts":"100","max_expanded_bytes":"16777216"}}`.
Supply `max_parts`, `max_part_bytes` and `max_expanded_bytes` as unsigned
decimal strings without leading zeroes; strings preserve exact 64-bit byte
counts. Omitted fields use native defaults: 10,000 ZIP members, 64 MiB for
one expanded member, and 256 MiB total expanded bytes. These are import
budgets, not limits on compressed file size or on retained previews.

### Edit part of a paragraph

Use the **paragraph** anchor and count Unicode scalar values in its original
text. Inspection returns `scalar_length` for candidate paragraphs and `null`
where scalar positions are not advertised. A length is not a guarantee that
a protected paragraph can be edited. In `A😀BC`, the emoji occupies `[1,2)` and the end position
is `4`. This batch replaces the emoji and appends a period, producing `AxBC.`:

```json
{
  "document": "<document>",
  "edits": [
    {"kind":"text-range","anchor":"<paragraph anchor>","start":1,"end":2,"text":"x"},
    {"kind":"insert-text","anchor":"<paragraph anchor>","offset":4,"text":"."}
  ]
}
```

Use `text:""` in a nonempty `text-range` to delete. Use `insert-text` with
nonempty text for a point insertion. Offsets can cross consecutive plain
text runs but cannot cross a paragraph, field or hyperlink. A combining mark
counts separately from its base character. In JavaScript use `Array.from(text)`
instead of UTF-16 string indices. The core rejects conflicting edits and
paragraphs with unsupported markup; see [the native range contract](EDITOR-PARITY.md#character-ranges-in-plain-body-paragraphs)
for formatting and boundary rules.

### Inspect and edit a footnote, endnote, or comment

Call `inspect_document` with `{"document":"<document>","view":{"kind":"entries","limit":10}}`.
The `entries` list contains footnotes, then endnotes, then comments in source
order. Each entry identifies its physical part, original string ID, kind,
note type or comment metadata, and indexed blocks. An unsupported block is
marked opaque. Reserved note entries are visible but do not have edit anchors.
If `next_cursor` is non-null, pass it as `view.cursor` on the next call. The
default page size is 25 entries; the accepted range is 1–100.

Copy the `anchor` from the entry itself, not its ID. A footnote and an
endnote can share the same ID. For example, to change a base comment's author:

```json
{
  "document": "<document>",
  "operation": {
    "kind": "edit-entry",
    "anchor": "<comment entry anchor>",
    "edit": {"kind":"comment-author","value":"Editorial team"}
  }
}
```

Send this to `preview_edits` as **one operation**, without an `edits` array.
It returns a retained preview and digest; `changes` is `null` because the
native entry editor returns document bytes without a change report.

| Entry edit | Input and effect |
| --- | --- |
| `insert-paragraph` | `index`, `text`: insert before an original block index; use the block count to append. |
| `replace-paragraph` | `index`, `text`: replace an eligible simple paragraph at that index. |
| `remove-paragraph` | `index`: remove an eligible paragraph; cannot remove the last one. |
| `comment-author` | `value`: set a base comment's author string. |
| `comment-initials`, `comment-date` | `value`: set the string or pass explicit `null` to clear it. |
| `delete-unused-note` | Remove an unreferenced, discardable ordinary note definition. |

An anchor identifies a candidate. The editor still protects references,
reserved note markers, rich content and threaded comments. Entry cursors and
anchors are valid only for the document that issued them. After reopening a
saved copy, inspect it again.

### Create or change a note reference

Call `inspect_document` with `view:{"kind":"note-references"}`. The
`note_references` page distinguishes `insertion-candidate` runs from direct
`reference` runs. Reference records include `note_kind` and the original
string `id`. Use the site anchor from this page, not the entry anchor from
the `entries` view.

```json
{
  "document": "<document>",
  "operation": {
    "kind": "edit-note-reference",
    "anchor": "<insertion candidate anchor>",
    "note_kind": "footnote",
    "edit": {"kind":"create","id":42,"text":"Source citation."}
  }
}
```

`create` and `attach` insert after a candidate run. `create` adds a new
definition (and creates the note part if necessary); `attach` uses an existing
unused definition. With a reference anchor, use `detach` to leave its
definition in place, `delete` to remove both when the definition is discardable,
`retarget` to point to an unused definition, or `renumber` to change both IDs.
`attach`, `retarget`, and `renumber` take `id` as an unsigned 32-bit integer;
for a stored ID like `"007"`, supply `7`. Set `note_kind` to `footnote` or
`endnote`. The server uses the related part, but cannot select a relocated
part with no entries. A successful preview has `changes: null`.

### Change direct paragraph or section layout

Call `inspect_document` with `view:{"kind":"layout"}`. Each `layout` row
identifies a direct paragraph by its body index or a section by its section
index. Its `layout` object contains only modeled direct values, not computed
formatting.

```json
{
  "document": "<document>",
  "operation": {
    "kind": "edit-layout",
    "edits": [
      {"anchor":"<paragraph anchor from layout>","field":"firstLineIndent","value":-240},
      {"anchor":"<section anchor from layout>","field":"marginLeft","value":1440}
    ]
  }
}
```

Use the lower camel case field names and the
[native layout values and units](EDITOR-PARITY.md#direct-paragraph-and-section-layout).
Each edit requires a `value`. Pass `null` to clear that field, `false` for
an explicit off flag, or omit the edit to leave the field alone. Paragraph
and section changes can share one layout transaction, but cannot be combined
with an ordinary batch in the same preview.

### Edit body blocks and section boundaries

Use compact inspection for paragraph anchors, and `view:{"kind":"sections"}`
for section anchors and original half-open body ranges. In the ordinary
`edits` array, use `{"kind":"block","anchor":"<anchor>","edit":{...}}`.

| Edit kind | Target and input |
| --- | --- |
| `insert-rich-paragraph` | Top-level edit (not nested under `block`); paragraph anchor and a `paragraph` declaration. |
| `replace-paragraph`, `remove-paragraph` | Paragraph anchor; replacement requires `paragraph`. |
| `replace-section` | A paragraph in the section; ordered `blocks`. The Rust editor requires the first direct paragraph of that section. |
| `split-section` | Section anchor; `offset` is an original section-relative block gap. |
| `merge-section` | Section anchor for the boundary to remove; the following section provides layout. |
| `keep-with-next` | Paragraph anchor; `value` is an explicit boolean. |
| `paragraph-flag` | Paragraph anchor; named direct `field` and explicit boolean or `null` to clear. |
| `section-flag` | First paragraph's anchor; named direct `field` and explicit boolean or `null` to clear. |
| `section-type` | First paragraph's anchor; `nextPage`, `nextColumn`, `continuous`, `evenPage`, `oddPage`, or `null` to clear. |

A split can add an empty paragraph to carry the new section boundary.

For example, insert a formatted paragraph after an existing one:

```json
{
  "document":"<document>",
  "edits":[{
    "kind":"insert-rich-paragraph",
    "anchor":"<paragraph anchor>",
    "paragraph":{
      "format":{"spacing_after":9},
      "inlines":[{"kind":"text","text":"Important","format":{"bold":true}}]
    }
  }]
}
```

These formatting sizes and spacings are in points, not the direct layout
view's stored units. The core rejects overlapping changes, unsafe boundaries,
or replacements that would discard protected content. For a section or
story block list, wrap each paragraph declaration as
`{"kind":"paragraph","paragraph":{...}}`.

The declaration accepts `text`, `tab`, `break`, `image`, `hyperlink`,
`fraction`, and `field` inlines. For a hyperlink, pass `target` as
`{"kind":"url","url":"https://example.org"}` or
`{"kind":"bookmark","name":"Bookmark1"}`, plus its displayed `children`.
Images take encoded `bytes`, not paths. A table block uses `kind:"table"`,
rectangular plain-text `rows`, and optional `column_widths` in twips. See
tool discovery for the typed members. Authoring annotations, nested and
formatted table cells, and some native inline types are not yet available
in these declarations.

### Change a field or a direct body inline

Use `view:{"kind":"body-inlines"}` to page through original paragraph inline
positions, including runs, hyperlinks, fractions, and opaque content. Use the
inline's anchor in the ordinary `edits` array:

```json
{"kind":"inline","anchor":"<run anchor>","edit":{
  "kind":"insert-after","inline":{"kind":"field","instruction":{"kind":"page"}}
}}
```

`replace` takes the same `inline` declaration. On an eligible hyperlink,
`remove-hyperlink` removes the wrapper and its displayed runs;
`hyperlink-tooltip` and `hyperlink-history` set a direct `value` or clear it
with `null`. Inspection retains the stored history spelling.

The `fields` view returns original coordinates, instruction text, simple
versus complex form, raw `dirty_xml` and `locked_xml` values, and the span's
inline count for a supported field. With its anchor, send an ordinary
`{"kind":"field","anchor":"<field anchor>","edit":{...}}`.
Use `instruction` with a string `value`, `dirty` or `locked` with a boolean
or explicit `null`, `cache` with an inline `children` list, `replace` with a
field `inline` declaration, or `remove`. Rust validates the original span,
cached content, and overlap with the rest of the batch.

### Move, add or remove a bookmark or comment range

Use `view:{"kind":"range-markers"}` to page through the original direct
body markers and comment references. Each `range_markers` row gives its
zero-based `body_index` and `inline_index`, `kind`, and stored `id` and
`name` strings (or `null` if absent). IDs are shown as stored, so `"+007"`
is edited with the numeric ID `7`. A row describes a physical marker, not
necessarily a valid or editable pair.

In the ordinary `edits` array, use inclusive endpoint anchors from
`body-inlines`. The endpoints may be the same inline or in different
paragraphs, and both refer to the original snapshot:

```json
{"kind":"paired-range","id":7,"bookmark":"OldPlace",
 "endpoints":{"first":"<first inline anchor>","last":"<last inline anchor>"}}
```

For a bookmark, use its exact existing name when moving or removing it.
Omit `bookmark` for a comment range; the numeric ID must identify one
existing comment definition (see `entries`). Omit `endpoints` to remove the
markers. For a comment this also removes the body reference, not the
definition. Neither optional field accepts explicit `null`. The editor
rejects ambiguous pairs, protected endpoints, and unsafe removals.

### Format runs in one physical part

Use `view:{"kind":"format-runs"}`. A run row has a physical `owner` anchor,
original coordinates, and direct formatting; a candidate row also has its own
run `anchor`. Main-body cell paragraphs, direct hyperlink children, and
effective headers, footers, notes and comments are included. Aliases of a part
refer to the same physical run.

```json
{
  "document":"<document>",
  "operation":{"kind":"edit-run-format","anchor":"<owner anchor>","edits":[
    {"anchor":"<run anchor>","edit":{"kind":"patch-format","set":{"bold":false},"clear":["font"]}}
  ]}
}
```

Use `set-bold` with a boolean `value`, or `set-toggle` with a `toggle` of
`bold`, `italic`, `strike`, or `hidden` and boolean or `null` `value`.
`patch-format` sets only supplied direct properties and clears only named
slots. Keep all runs in one physical owner; the native transaction may
reject protected content even for a candidate anchor. The scoped result has
`changes: null`. To mix a direct body run format change into an ordinary
batch, use the `body-inlines` anchor with `edit.kind:"format"` and the same
action under `edit.action`.

### Edit table geometry and content

Use `view:{"kind":"body-tables"}` for direct body tables in an ordinary
`table-structure` edit. Use `view:{"kind":"physical-tables"}` to find nested
tables or tables in header/footer/note/comment parts. For a physical
transaction, use `operation.kind:"edit-part-tables"` with the physical owner
`anchor` and `edits`, each with that owner's table `anchor` and an `edit`.
The physical view's `path` describes original XML element positions; it is
not an edit handle. Results retain direct XML property values and original
grid widths.

Both forms support `layout`, row/cell/column insertion and removal,
`replace-cell` with `blocks`, and `set-property`/`clear-property`. Property
edits take a typed `site` and `field`; setting copies that field from a
`source` table declaration. Only the ordinary body transaction supports
whole-node `replace-node` or `remove-table`. Positions are original
zero-based coordinates, with insertion before the supplied index or at
the count to append. Native validation protects grid merges and opaque
content.

Use `insert-rich-table` with a paragraph anchor and a `table` declaration
to insert a new body table. Supply `rows`, each with `cells`, each with
`blocks`; optional `properties` at each level describe direct settings.
Use `{"kind":"rich-table","table":{...}}` as a block to nest a table in a
cell or supply one to a section or story edit. Grid widths are in twips,
row heights and margins in points, and border sizes in eighth-points.
Row and cell control-wrapper declarations are not exposed here.

### Propose and review tracked changes

Use compact inspection for proposal anchors. Send a standalone
`edit-revision` operation, for example:

```json
{
  "document": "<document>",
  "operation": {
    "kind":"edit-revision",
    "anchor":"<direct body run anchor>",
    "edit":{"kind":"replace-text","text":"Revised wording","author":"Editorial team"}
  }
}
```

| Proposal edit | Target and extra arguments |
| --- | --- |
| `insert-text-after`, `replace-text` | Direct body run; `text`. |
| `delete-text` | Direct body run. |
| `move-text` | Source run; `paired_anchor` on the operation for the destination run, `name` and required `date` on the edit. |
| `set-bold` | Direct body run; `bold` boolean. |
| `insert-paragraph-before` | Direct body paragraph; `text`. |
| `delete-paragraph` | Direct body paragraph. |

All proposals need `author`. Other proposals may omit `date`; if supplied,
use a UTC timestamp such as `"2030-02-03T04:05:06Z"`.
Save and reopen the proposed copy, then call `inspect_document` with
`view:{"kind":"revisions"}`. Pending revisions list literal metadata and
original inline paths. Use the returned anchor with `review` for eligible
inline wrappers or run property changes and `review-paragraph` for an eligible
paragraph mark. Set `action` to `accept` or `reject`. Paired move review
requires both wrapper anchors; pass the second as `operation.paired_anchor`.
Some nested revisions are read-only; the core checks anchors, pairing,
protected ranges and paragraph boundaries before staging any changes.

### Inspect and edit headers and footers

Call `inspect_document` with `view:{"kind":"stories"}` for the six logical
slots per section: default, first-page and even-page headers and footers.
Each row has a slot `anchor`, `section_index`, `kind` (`header` or `footer`)
and `variant` (`default`, `first` or `even`). The `owner` is `null` for a
missing slot. When several slots share a package part, their physical owner
anchor and run anchors match; their slot anchors are still distinct.

- Use the slot anchor of an existing part with the ordinary `header-image`
  edit, including for footers. `shared` changes all aliases that use the
  part; `section-local` only changes the selected section slot.
- Use `operation:{"kind":"edit-story-text","anchor":"<owner anchor>","edits":[{"anchor":"<run anchor>","text":"Updated"}]}`
  to edit eligible direct text runs in one physical part. Hyperlink-child
  runs are visible but not targets of this operation.
- Use `operation:{"kind":"edit-story-reference","anchor":"<slot anchor>","edit":{"kind":"create","paragraphs":["New footer"]}}`
  to create an explicit reference from a missing or inherited slot.
  Use `edit:{"kind":"link","anchor":"<owner anchor>"}` to link to an
  inspected physical part of the same kind. Use `edit:{"kind":"remove"}` to
  remove an explicit reference and resume inheritance, without deleting the
  package part.
- Use `create-rich` with `blocks` instead of plain `paragraphs` to author
  formatted content when creating a story. To splice an existing story, use
  `operation:{"kind":"edit-story-blocks","anchor":"<slot anchor>","edit":{"mode":"section-local","index":0,"remove":0,"blocks":[...]}}`.
  The `index` and `remove` count original blocks, including opaque blocks.
  The server does not discard opaque content to make a splice succeed.

Scoped story edits return `changes: null`; image edits include a record of
the physical source/output parts and affected sections. The compact view
still returns only its original default and first-page header rows.

For notes and comments, use `view:{"kind":"entry-runs"}` instead. It returns
rows with lexical entry IDs, a physical `owner`, and runs with original
block and inline indices. Pass that owner and its direct run anchors to the
same `edit-story-text` operation. You can edit runs in several entries in
one part, but cannot mix parts. Reserved notes and note markers do not
receive run-edit anchors.

### Edit metadata and direct document defaults

Call `inspect_document` with `view:{"kind":"resources"}` to read `metadata`,
`document_defaults`, and the legacy `default_alignment`. To look up one
named string or boolean custom property, add `custom_property` to the view.
The result is `null` for an absent name; `false` and `""` are stored values.
Defaults are direct settings, not effective formatting after style inheritance.
For an independent query, use
`view:{"kind":"resource-value","query":{"kind":"metadata","field":"title"}}`.
It returns `value` without reading unrelated properties. Other query kinds
are `document-defaults`, `default-alignment`, and `custom-property` (with
`name`).

Use `kind:"resource"` in the ordinary `edits` array. Resource edits need no
anchor and may be mixed with body edits in one preview. For example:

```json
{
  "document":"<document>",
  "edits":[
    {"kind":"resource","edit":{"kind":"metadata","field":"company","value":"Example Co."}},
    {"kind":"resource","edit":{"kind":"default","field":"keepTogether","value":false}},
    {"kind":"resource","edit":{"kind":"custom-property","name":"Final","value":false}}
  ]
}
```

Every resource edit requires `value`: omit the edit to retain the current
property; pass `null` to clear it. The native metadata name `comments` aliases
`description`. The `core-title`, `core-subject` and `default-alignment`
edits retain the native shorthand fields. See
[metadata and defaults](EDITOR-RESOURCES.md) for value domains and units.

### Save or continue editing

Every preview starts from the opened or created document. Put edits that
should apply together in one batch, using anchors from inspection of that
document. A rejected batch leaves earlier previews and anchors usable.
To continue editing a saved result, open that copy and inspect it for new
anchors. Changes to the source file on disk do not alter a live snapshot.
To stage the native empty-batch result, use `operation:{"kind":"no-op"}`.
This returns a retained preview with empty change arrays. It preserves
package payloads but does not promise the source ZIP's exact encoding.

`save_copy` writes the selected preview's retained bytes and returns their
SHA-256 as `output_snapshot`. It refuses an existing destination. Choose
another name and retry with the same preview. An I/O failure after creating
the destination may leave a partial new copy.

One session can retain 16 documents and 64 previews (256 MiB of encoded
preview bytes in total). A created document's initial preview counts toward
those limits. Close documents to release capacity. Closing a document or
restarting the server invalidates its handles; open or create a document
again and inspect it before retrying.
