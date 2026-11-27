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
and save as above. The ordinary `edits` batch can mix supported body, table,
inline, control, resource, and story-image edits. Separately scoped
transactions such as revision review and physical story text use
`operation` instead. The sections below show the inspection views and edit
forms for each scope.

To override archive read budgets for one open, pass, for example,
`{"path":"input.docx","limits":{"max_parts":"100","max_expanded_bytes":"16777216"}}`.
Supply `max_parts`, `max_part_bytes` and `max_expanded_bytes` as unsigned
decimal strings without leading zeroes; strings preserve exact 64-bit byte
counts. Omitted fields use native defaults: 10,000 ZIP members, 64 MiB for
one expanded member, and 256 MiB total expanded bytes. These are import
budgets, not limits on compressed file size or on retained previews.

### Find text without reading the entire body

Call `search_document` on a live handle from `open_document` or
`create_document`, for example:

```json
{"document":"<document>","query":"Quarterly report","limit":10}
```

Search uses the original snapshot, not a preview or the current file at the
source path. It does not edit the snapshot, issue edit anchors, or write
files. It matches literal, case-sensitive text (1–128 Unicode scalar values)
without trimming or normalization. Matches can span differently formatted
runs and ordinary hyperlinks, but not paragraphs. Only direct main-body
paragraphs are searched; table cells, headers, footers, notes, comments, and
paragraphs wrapped in a control or revision are not searched.

This is the same text projection as the compact body view, not rendered
Word text. It includes ordinary run and hyperlink text, tabs and line breaks.
It does not search inside tracked-change or opaque inline wrappers. A match
can join text on either side of content omitted from the projection.

The `hits` list contains non-overlapping matches ordered by body position
and then by position within each paragraph. Each hit has a zero-based
`body_index` (the `index` in the compact body view), inclusive `start` and
exclusive `end` Unicode scalar offsets in that paragraph's projected text,
and a `snippet` with up to 24 context scalars on each side of the match.
`snippet_start` is the scalar offset where that snippet begins. These search
coordinates are not edit permission; use `inspect_document` for edit anchors.
On a miss the list is empty, no document text is returned, and `next_cursor`
is `null`.

Each page returns at most `limit` hits (default 25, range 1–100). Pass
`next_cursor` as `cursor` with the same document handle and exact query to
continue, optionally changing `limit`. `next_cursor: null` means the search
is exhausted. The server keeps up to 256 distinct search cursors per
snapshot and discards the oldest on overflow; omit `cursor` to restart.
Search uses the snapshot's existing import limits and adds no document size
cap or scan cutoff.

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
`endnote`. The server uses the relationship-selected part even when it is
relocated and has no entries. A successful preview has `changes: null`.

### Change direct paragraph or section layout

Call `inspect_document` with `view:{"kind":"layout"}`. Each `layout` row
identifies a direct paragraph by its body index or a section by its section
index. Its `layout` object contains only modeled direct values, not computed
formatting. Set `scope` to `"paragraphs"` or `"sections"` to inspect just
that kind of target. Paragraph-only discovery does not read the section
catalog, so unrelated section errors do not hide paragraph edit anchors.
A cursor stays with the scope that issued it.

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
Images take encoded `bytes`, not paths. Optional `source_name` sets named
image metadata. Tabs, breaks, and inline images accept `format`; hyperlinks
accept `tooltip` and `history`, where `false` is distinct from omission.
A plain table block uses `kind:"table"`, rectangular text `rows`, and
optional `column_widths` in twips. `rich-table` accepts nested blocks and
modeled table, row, and cell formatting. A rich row may be empty if another
row establishes the grid. The native editor validates each declaration for
its edit scope; general authoring annotations are not fragment inputs.

### Change a field or a direct body inline

Use `view:{"kind":"body-inlines"}` to page through original paragraph inline
positions, including runs, hyperlinks, fractions, and opaque content. Use the
inline's anchor in the ordinary `edits` array:

```json
{"kind":"inline","anchor":"<run anchor>","edit":{
  "kind":"insert-after","inline":{"kind":"field","instruction":{"kind":"page"}}
}}
```

The `inline_handle` is for readback, not editing. Pass it to
`view:{"kind":"inline-at","inline_handle":"<handle>"}` to retrieve a typed
node, including an addressable child of a hyperlink or revision. Handles
belong to the open snapshot and are distinct from edit anchors. The
`entry-inlines`, `story-inlines`, and `table-cell-inlines` views issue the
same kind of read handles.

`replace` takes the same `inline` declaration. On an eligible hyperlink,
`remove-hyperlink` removes the wrapper and its displayed runs;
`hyperlink-tooltip` and `hyperlink-history` set a direct `value` or clear it
with `null`. Inspection retains the stored history spelling.

The `fields` view returns original coordinates, instruction text, simple
versus complex form, raw `dirty_xml` and `locked_xml` values, and the span's
inline count for a supported field. To read one original inline position
directly, use `view:{"kind":"field-at","anchor":"<body paragraph anchor>","inline_index":1}`.
The response's `field` contains those five values. A position that does not
start a supported field returns the native inspection error. For editing,
use the anchor from the `fields` page in an ordinary
`{"kind":"field","anchor":"<field anchor>","edit":{...}}`.
Use `instruction` with a string `value`, `dirty` or `locked` with a boolean
or explicit `null`, `cache` with an inline `children` list, `replace` with a
field `inline` declaration, or `remove`. Rust validates the original span,
cached content, and overlap with the rest of the batch.

For a deeply nested field, use an inline `field-tree` declaration so the
request does not need a deeply nested JSON object:

```json
{"kind":"field-tree","nodes":[
  {"instruction":{"kind":"page"},"children":[{"kind":"field","node":1}]},
  {"instruction":{"kind":"num-pages"},"children":[{"kind":"text","text":"1"}]}
]}
```

Node `0` is the root. References point to later nodes, each used exactly
once. Flat cache children can be fields, hyperlinks, text, tabs, or breaks.
Hyperlink children can be text, tabs, or breaks, not fields. The request
is bounded to 256 container levels, 16,384 nodes and 16,384 child entries, counting
hyperlink children, before native validation. The original `field`
declaration remains available for shallow trees.

### Inspect inlines in a header, footer, or table cell

For an existing header or footer, take `owner.anchor` from `stories`, not
the logical slot's `anchor`. Shared or inherited slots for the same part
use the same owner. To read one direct paragraph, pass
`view:{"kind":"story-inlines","anchor":"<owner>","block_index":0}`.

For a direct body table cell, take the table anchor from `body-tables`:

```json
{"kind":"table-cell-inlines","anchor":"<table>","row_index":0,
 "cell_index":0,"block_index":0,"limit":25}
```

Rows and cells use physical indices, not logical grid columns. Block
indices include opaque blocks, such as a nested table in a cell. Select a
paragraph block. Both views return ordered `inlines` and a `next_cursor`
bound to the same original paragraph. Runs expose ordered text, tabs,
breaks, note references, and opaque item names, plus direct formatting and
modeled drawing or revision data where available. Use the existing
`format-runs` view to discover editable run anchors.

### Change a drawing, chart, or fraction

A `body-inlines` run has a `drawing` object when the native editor recognizes
an embedded picture, shape, or chart. It shows the occurrence's placement,
extent in EMUs, accessibility attributes and relationship ID, plus modeled
shape or picture properties. Crop readback is in thousandths of a percent
and rotation in 1/60000 degree units. An explicit `false` flip is distinct
from an absent flip. A simple fraction appears as its own row.
If an imported extent is outside the safe JSON integer range, use the
decimal string in `width_emu_exact` or `height_emu_exact` instead of the
corresponding numeric field.

For one supported chart, inspect
`view:{"kind":"chart-series","anchor":"<body-inlines anchor>"}` to read its
ordered `categories` and `values`. This is a separate lookup: a chart whose
series cannot be decoded does not prevent other inline rows from being read.
Update a workbook-free column series in an ordinary inline edit:

```json
{"kind":"inline","anchor":"<body-inlines chart anchor>","edit":{
  "kind":"drawing-property","property":"chart-series",
  "inline":{"kind":"column-chart","width":100,"height":75,
            "categories":["North","South"],"values":[12,8]}
}}
```

The donor declaration's width and height are in points; `chart-series`
does not copy them. A changed series forks the chart part for the selected
occurrence, leaving others that shared the old part unchanged.

Other `drawing-property` groups are `size`, `placement`, `crop`, `transform`,
`accessibility`, `shape-geometry`, `shape-fill`, `shape-outline`, and
`shape-text`. Use a donor of the same drawing kind. An `image` declaration
takes encoded `bytes` and optional `alt_text` and `transformation`. The
transformation takes dimensions and offsets in points, rotation in degrees,
crop in percentages, and optional floating `anchor` properties. A `shape`
declaration takes a `properties` object with optional geometry, dimensions,
fill, outline, plain text, text format, and floating anchor. See tool
discovery for the full declaration schema.

Use `drawing-description` with a string `value` or `null`, `image-asset`
with encoded `bytes` to redirect just one picture, and `remove-object` to
remove an eligible object. `insert-after` and `replace` take the same
declarations. Reopen the output to patch a second group on the same
occurrence. Protected or opaque changes are rejected by the native editor;
shapes, charts and fractions are edited only in the direct body. Pictures
in notes and comments use the separate entry operation below.

### Edit inlines in notes and comments

Use `view:{"kind":"entry-inlines"}` to page through direct inlines in
footnotes, endnotes, and comments. In `entry_inlines`, each row identifies
its physical `owner`, entry (kind, original catalog index, and stored ID),
block and inline indices, and native inline projection. A top-level
revision row carries the stored revision identifier in `revision_id`;
its `id` still identifies the entry. Separator and
continuation notes and reference marks are inspectable but do not have inline
edit anchors. The native editor still checks each issued target for protection
and eligibility.
Use `entry-runs` instead for the existing text-only `edit-story-text` flow.

Use the owner anchor and target inline anchors from `entry-inlines` in a
single `preview_edits.operation`:

```json
{"kind":"edit-entry-inlines","anchor":"<entry owner anchor>","edits":[
  {"anchor":"<entry picture anchor>","edit":{"kind":"drawing-description","value":"New caption"}}
]}
```

In one physical part you can insert or replace an eligible inline with
text, a tab, break, external hyperlink, or embedded image, and format
eligible direct runs. Eligible hyperlink child runs have their own edit
anchors, but a batch using any of those anchors must contain only `format`
edits. Direct shape and chart runs in an imported entry likewise support
format-only edits, not drawing changes. On pictures use `remove-object`, `image-asset`,
`drawing-description` (including `null` to clear), or `drawing-property`
with an image donor for `size`, `placement`, `crop`, `transform`, or
`accessibility`. A new relationship belongs to the entry part, even when
that part was relocated. Ordinary body edits and other owners cannot be
included in this operation. The preview returns `changes: null`, and a
reopened output supplies fresh anchors.

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

### Inspect and edit content controls

Use `view:{"kind":"controls"}` for main-story controls, including those in
tables. The `controls` rows report the native context, literal ID and lock,
tag and alias, binding status, type, and stored list or multiline properties.
Use `view:{"kind":"control-sites"}` for eligible original wrapping sites.
The `control_sites` rows have a context (`block`, `inline`, `row`, or `cell`),
text label and anchor. IDs, tags and text labels are not addresses.

Use `view:{"kind":"story-controls"}` to read controls in physical headers,
footers and ordinary note/comment entries. The `story_controls` rows identify
the owner part and, where applicable, the literal entry ID and physical entry
index. Shared parts are listed once; these rows do not issue edit anchors.

In an ordinary `preview_edits.edits` batch, use `kind:"control"` with a typed
`edit`. Wrap a single site by omitting `last`, or wrap an inclusive range of
ordered siblings from the same snapshot:

```json
{"kind":"control","edit":{"kind":"wrap","first":"<site anchor>",
 "last":"<site anchor>","tag":"Reviewed"}}
```

Other actions take an `anchor` from `controls`: `patch` sets supplied `tag`,
`alias` or single-line `text`; `unwrap` promotes the existing payload;
`define` replaces all modeled properties. Supply a `text` declaration of
kind `rich`, `plain` (optional `multi_line`), or `drop-down` (`items` and
`selected`). Omit `id`, `tag`, `alias` or `lock` to clear the corresponding
property; supported lock values are `unlocked`, `control`, `content`, and
`both`. `choice`
selects a literal `selected` value, optionally replacing the ordered `items`
(`display_text` and `value` pairs). Use `replace-blocks` with `blocks` or
`replace-inlines` with `inlines` for a nonempty rich payload matching the
control's context. Both reuse the existing block and inline declarations.

The editor rejects unsupported, bound, locked or protected edits and
overlapping targets. Reopen a saved preview to inspect the new controls or
sites; a preview does not issue new anchors or change the source snapshot.

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
tables or tables in header/footer/note/comment parts. To avoid unrelated
catalogs, set `scope` to `{"kind":"main"}` for the main part, or to
`{"kind":"story","anchor":"<story owner>"}` for an inspected header or
footer. Use `{"kind":"entry","anchor":"<entry owner>"}` to limit the
result to the note/comment part identified by `entry-runs` or `entry-inlines`.
The native entry query still validates the catalogs it uses. Cursors stay
with their original scope. For a physical
transaction, use `operation.kind:"edit-part-tables"` with the physical owner
`anchor` and `edits`, each with that owner's table `anchor` and an `edit`.
The physical view's `path` describes original XML element positions; it is
not an edit handle. Results retain direct XML property values and original
grid widths.

The `body-tables` `children` tree preserves ordered row/cell siblings and
opaque blocks. Use the original filtered `rows`/`cells` indices for table
edits. The body-only `view:{"kind":"body"}` returns body paragraphs and
tables even when an unrelated story cannot be inspected; its edit anchors
have the same coordinates as the compact view.

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
The native editor does not accept row or cell control wrappers in table
fragments. Use the separate control wrapping transaction on an eligible
existing table site.

For deep nesting, use `{"nodes":[...]}` instead of the recursive table
declaration. Each node has the same `rows` and optional `properties`. Start
with node `0`; put `{"kind":"table-ref","node":1}` in a cell's `blocks`
where the nested table belongs. References must point forward and use
each later node exactly once. Paragraph and plain table blocks remain
available. A flat declaration allows up to 1,024 nodes, 84 table levels,
and 16,384 row, cell, and block entries. Across the declaration, it may
emit up to 65,536 grid columns and 16,384 physical cells, including padding.
a spanning cell counts once. A preview rejects an edit if the resulting
Word part exceeds the native XML read depth; the containing story and
cells contribute to that depth. Every constructed table fragment (including
a nested one) requires at least one cell. A `set-property` donor must
have at least one row, but its rows may contain no cells because only
the selected property is copied.

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
original inline paths. For full typed child values, including formatting
and hyperlink properties, read the original node through `body-inlines`
and `inline-at`. Use the review anchor from `revisions` with `review` for eligible
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
Story discovery does not validate unrelated section layout. Shared edits
can still succeed when a section-local edit must reject ambiguous layout.

For effective default headers only, use `view:{"kind":"default-headers"}`.
This independent native reader omits sections without a default header and
can still return valid default headers when an unrelated footer or
first-page header is malformed. Its paragraph blocks contain typed inlines
and read handles usable with `inline-at`. To edit the header, use the
row's `owner.anchor` with `edit-story-text` or `edit-run-format`. Use
`run_targets[].anchor` for text and `run_targets[].format_target.anchor`
for formatting. For `header-image` or `edit-story-blocks`, use
`image_slot.anchor`, which identifies the selected section, not the shared
physical owner. Edit eligibility still follows native guards.

- Use the slot anchor of an existing part with the ordinary `header-image`
  edit, including for footers. `shared` changes all aliases that use the
  part; `section-local` only changes the selected section slot.
  Pass either `bytes` or `image`. The latter takes `bytes` and optional
  `source_name`, `transformation`, and `alt_text`. Story image insertion is
  inline-only; Rust rejects a floating position.
- Use `operation:{"kind":"edit-story-text","anchor":"<owner anchor>","edits":[{"anchor":"<run anchor>","text":"Updated"}]}`
  to edit eligible direct text runs in one physical part. Hyperlink-child
  runs are visible but not targets of this operation.
- Use `operation:{"kind":"edit-story-reference","anchor":"<slot anchor>","edit":{"kind":"create","paragraphs":["New footer"]}}`
  to create an explicit reference from a missing or inherited slot.
  The `sections` view also issues the six logical anchors in each section's
  `story_slots` without reading story bodies. Use those anchors for
  reference edits when an unrelated story cannot be inspected.
  Use `edit:{"kind":"link","anchor":"<owner anchor>"}` to link to an
  inspected physical part of the same kind, including a default-header
  owner from `default-headers`. To find related but unused
  parts, call `view:{"kind":"linkable-stories","story_kind":"footer"}` (or
  `"header"`) and use a returned anchor. Set `include_referenced:true` to
  include parts already used by a slot, without inspecting stories of the
  other kind. Cursors are scoped to the story kind and this option.
  Use `edit:{"kind":"remove"}` to remove an explicit reference and resume
  inheritance without deleting the part, so it remains available for relinking.
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

### Inspect and edit imported styles

Use `view:{"kind":"styles"}` to inspect the relationship-selected styles part.
The paged `styles` list is in stored order. `part_name` is `null` if the
document has no styles relationship. Each row shows its stored ID, kind,
display name, base/next/linked IDs, and `property_xml` for direct property
groups and table regions. This is not computed formatting. Duplicate or
missing IDs do not issue edit anchors.

In the ordinary `preview_edits.edits` array, send `kind:"style"` with an
`edit` of kind `add`, `replace`, `patch`, `field`, `region`, `remove`, or
`retarget`. Except for `add`, these use the original style anchor, not the
display name or ID. `retarget` takes a second anchor as `replacement`;
it must be the only style operation in its batch.

```json
{"kind":"style","edit":{"kind":"field","anchor":"<style anchor>",
 "field":"jc","value":"center"}}
```

`patch` sets the supplied `bold` or `italic` flag. `field` takes a
[native field key](EDITOR-PARITY.md#imported-style-edits) and a stored string
value (or `null` to clear), with an optional OOXML region such as `firstRow`.
`region` replaces the entire named conditional table region with a
declaration, or clears it with `null`. Style definitions support paragraph,
character, table, and numbering kinds. Paragraph declarations take direct
paragraph `format` and optional `text` formatting; table declarations use
a `regions` list of `{region, format}` pairs. Numbering styles bind to an
existing `numbering_id`. See tool discovery for the typed definitions.

An `add` can create a missing catalog or introduce a style used by body
insertion in the same preview. The native editor rejects conflicting style
IDs, unsafe replacement or removal of referenced definitions, and reference
migrations it cannot prove safe. Reopen a saved preview to use anchors for
new styles.

### Inspect and edit numbering

Use `view:{"kind":"numbering-abstracts"}` and
`view:{"kind":"numbering-instances"}` for paged `abstracts` and `instances`.
The native readback includes stored levels, start and replacement overrides,
and numbering-style links. It does not compute inherited numbering or the
label Word will display. Each cursor is valid only for its own view and
document. A missing numbering part returns empty lists.

For narrower reads, use `resource-value` with `query.kind:"abstract-starts"`
or `query.kind:"numbering-instances"`. These native readers do not depend on
the full catalog or style-link readback. To read one direct body paragraph's
assignment, pass its compact inspection anchor to
`view:{"kind":"paragraph-numbering","anchor":"<paragraph anchor>"}`. `value`
is `{id, level}` or `null` if no assignment is directly stored.

Use `kind:"numbering"` in the ordinary edit array:

```json
{"kind":"numbering","edit":{"kind":"paragraph","anchor":"<paragraph anchor>",
 "value":{"id":90,"level":0}}}
```

`add-abstract` and `replace-abstract` take an `abstract_id` and `levels`.
Each level specifies `level` (0–8), `start`, OOXML `format`, and label `text`,
and may include direct paragraph and label formatting. Use `abstract-level`
to replace or remove a level, and `abstract-start` to change just its start.
`add-instance` and `relink-instance` take `num_id` and `abstract_id`;
`replace-instance` replaces a link and all direct overrides via a
`definition` with `id`, `abstract_id`, `start_overrides`, and `level_overrides`.
Omit `levels` and `inherit_style` from this concrete definition; define levels
on the abstract instead.
Use `instance-start` and `instance-level` to change the two override types
independently. Use `null` in a required `value` to clear it. `remove-instance`
and `remove-abstract` use the respective numeric IDs.

Definitions, numbering styles, and paragraph assignments can use IDs staged
in the same preview, including when a numbering part is first created.
The native editor rejects conflicting edits and removals that
would leave live references.

For other physical paragraphs, use
`view:{"kind":"numbering-paragraphs","scope":"cells"}` for direct paragraphs
in direct body tables, `scope:"stories"` for effective headers and footers,
or `scope:"entries"` for notes and comments. Each scope has its own cursor.
The `paragraphs` rows include text, original coordinates, and a direct
numbering `value`. The same part shared by multiple sections is listed just
once. A row with `unavailable` instead of `value` and `anchor` could not be
read or addressed as an eligible native paragraph. Nested tables are not
traversed.

Send the issued anchor with `physical-paragraph` in the ordinary batch:

```json
{"kind":"numbering","edit":{"kind":"physical-paragraph",
 "anchor":"<physical paragraph anchor>","value":{"id":90,"level":0}}}
```

As with direct body numbering, `null` clears the assignment. A batch can
stage a list, assign it across several physical owners, and edit other
disjoint body content. A table structure edit cannot share the same table
as a cell numbering edit. Reopen a saved result to get fresh anchors.

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
Tool failures include `category`, `message`, and a suggested `action`.
Native diagnostics from reading a document, resolving a declaration, or
applying an edit can also include a stable `rule`, `path`, and unformatted
`detail`. A native invalid XML text
character also reports `character` and its zero-based UTF-8 `byte_offset`.
Those fields are optional; for example, filesystem errors may have no native
editor diagnostic.
To continue editing a saved result, open that copy and inspect it for new
anchors. Changes to the source file on disk do not alter a live snapshot.
To stage the native empty-batch result, use `operation:{"kind":"no-op"}`.
This returns a retained preview with empty change arrays. It preserves
package payloads but does not promise the source ZIP's exact encoding.

`save_copy` writes the selected preview's retained bytes and returns their
SHA-256 as `output_snapshot`. It refuses an existing destination. Choose
another name and retry with the same preview. An I/O failure after creating
the destination may leave a partial new copy.

To update the file you opened, call `save_in_place` with `document` and
`preview` (no output path). It checks that the source still matches the
original SHA-256 and replaces it with the exact preview bytes using a temporary
file in the same directory. It returns the canonical output path and
`output_snapshot`. Missing or changed sources, files opened through a
symbolic link at the filename, and in-memory documents are refused.

The live handle still describes the original snapshot. Reopen the saved file
to make further edits. Replacement does not update other hard links or
preserve all filesystem metadata. The snapshot check is not a lock against
other writers; close external editors before saving.

One session can retain 16 documents and 64 previews (256 MiB of encoded
preview bytes in total). A created document's initial preview counts toward
those limits. Close documents to release capacity. Closing a document or
restarting the server invalidates its handles; open or create a document
again and inspect it before retrying.
