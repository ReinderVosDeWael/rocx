# JSON editor CLI

Run `cargo run --release --bin rocx < request.json`. The CLI reads one JSON
object from stdin and writes one JSON object to stdout. Success exits 0 with
`{"ok":true,"result":...}`; failure exits nonzero with `{"ok":false,"error":"..."}`.
Paths are relative to the process working directory. Unknown request and edit
fields are errors.

## Inspect and address

`{"command":"inspect","source":"input.docx"}` returns `snapshot`, `body` and
`headers`. Use addresses from this response in the following requests:

- Direct paragraphs report `type:"paragraph"`, numeric `target`, projected
  `text`, `alignment_xml`, `has_opaque_content` and `runs`. Each direct run
  reports its original `inline_index` and its text for a single text item
  (`null` otherwise). Indices count all inline nodes, including opaque nodes;
  do not use an offset into the filtered `runs` list.
- Direct tables report `type:"table"`, `index`, `target` and `cells`. Each
  physical cell reports `row_index` and `cell_index`. Use those numbers with
  the enclosing table target. They are source child indices, not worksheet
  coordinates; grid spans and merges aren't expanded.
- Content controls report `type:"content-control"`, `index`, `tag`, `alias`
  and `lock`. Other unsupported blocks report `type:"opaque"`, `index` and
  `name`. Neither supplies an edit target.
- `headers` lists existing default and first-page header slots. Each supplies
  a `target` object (`section_index`, `variant`), an `inherited` flag and the
  physical `part`. Different slots can share one part. If story inspection
  fails, `headers_error` replaces `headers`; body inspection remains available.

All positions are zero-based in the **original** snapshot. An address identifies
a candidate: the core still checks whether editing it is safe.

## Preview and save

For a document that reported the corresponding targets:

```json
{
  "command": "preview",
  "source": "input.docx",
  "snapshot": "COPY THE DIGEST FROM INSPECT",
  "edits": [
    {"target": 0, "alignment": "center"},
    {"kind": "run-text", "target": 0, "inline_index": 0, "text": "Revised"},
    {"kind": "insert-paragraph", "target": 0, "text": "Following paragraph"},
    {"kind": "cell-alignment", "target": 1, "row_index": 0, "cell_index": 0, "alignment": "bottom"}
  ]
}
```

To save, change `command` to `save-copy` and add `"output":"copy.docx"`.
Preview and save independently resolve the same batch and return the same
change report. Save encodes the package before exclusively creating a new
file. An existing destination is an error; a write error can leave an incomplete
new file. The source is never opened for writing.

| `kind` | Required fields besides `kind` | Meaning |
| --- | --- | --- |
| `alignment` | `target`, `alignment` | Direct paragraph alignment. The legacy spelling without `kind` is also accepted. |
| `insert-paragraph` | `target`, `text` | Insert one plain-text declarative paragraph after the paragraph target. |
| `insert-table` | `target`, `rows` | Insert after the paragraph target. `rows` is a nonempty rectangular array of string arrays, e.g. `[["A","B"]]`; each cell contains one paragraph. |
| `header-image` | `target`, `mode`, `bytes` | Append an inline image to the selected header. Supply the header target object, `shared` or `section-local`, and a complete image as an array of integer bytes (0–255). The shared authoring image decoder determines format and dimensions. |
| `run-text` | `target`, `inline_index`, `text` | Replace one supported single-text direct run in the target paragraph. No tabs or line breaks. |
| `run-remove` | `target`, `inline_index` | Remove one supported direct paragraph run; the core refuses removal that could discard opaque semantics. |
| `cell-alignment` | `target`, `row_index`, `cell_index`, `alignment` | Set one physical cell's vertical alignment: `top`, `center`, `bottom` or `both`. |

Paragraph alignments: `left`, `center`, `right`, `justify`, `distribute`,
`justify-medium`, `justify-high`, `justify-low`, `thai-justify`. Strings,
indices, byte arrays and target objects must have their indicated JSON types.
Insertions do not shift other targets. Invalid addresses, conflicting edits,
unsupported declarations and unsafe edits reject the whole transaction.

The result includes the source `snapshot` and `output_snapshot` (SHA-256 of the
complete encoded output, including for preview). Ordinary batches return:

- `changes`: legacy paragraph alignment records (`target`, `before`, `after`;
  `after` retains Rust spelling, e.g. `Center`).
- `paragraph_insertions`: original paragraph target numbers.
- `table_insertions`: `target`, `rows`, `columns`.
- `run_text` / `run_removals`: `target`, `inline_index`, `before`; replacements
  also include `after`.
- `cell_alignments`: `target`, `row_index`, `cell_index`, `before`, `after`.
- `header_images`: `mode`, `source_part`, `output_part`, `affected_sections`,
  `restored_section` (nullable).
- `block_changes`, `inline_changes`: original block indices and `[target,
  inline_index]` pairs. Scalar ranges report per-run changes in `run_text`.
  Core operations without individual change records are covered by
  `output_snapshot`; inspect the saved copy to read their new values.

Each list follows the core's original document/section order; unchanged values
are omitted. `before` on an alignment is the original direct XML value or `null`.

## Snapshot and extension contract

`snapshot` is lowercase SHA-256 of the complete input DOCX bytes. Every preview
or save reads the source once, checks the digest, then binds addresses against
that same byte buffer in a newly opened core snapshot. Identical bytes at another
path are interchangeable. Any byte change, even a ZIP metadata change, invalidates
the digest. This is change detection, not authentication or a lock on a path.

## Extended ordinary batch

The following `kind` values can be combined with the seven above in one
`ImportedDocument::preview_edits` call. All coordinates refer to `inspect`.

| `kind` | Fields besides `kind` |
| --- | --- |
| `text-range` | Paragraph `target`, `start`, `end`, `text`. Replace `[start,end)` Unicode scalar values; `""` deletes. |
| `text-insertion` | Paragraph `target`, original Unicode scalar `position`, nonempty `text`. At a run boundary, formatting comes from the following run; at the end it comes from the last run. |
| `replace-paragraph`, `remove-paragraph`, `keep-with-next` | Paragraph `target`; additionally `text` for replacement, boolean `value` for keep-with-next. |
| `replace-section` | `target` (first direct paragraph), `blocks`. |
| `split-section`, `merge-section` | Section `target` from `sections`; additionally a section-relative original block `offset` for split. Merge consumes a nonfinal boundary. The core rejects edits overlapping affected sections. |
| `paragraph-flag`, `section-flag` | Paragraph `target` (section start for section flags), `flag` (Python snake_case name), `value` (`true`, `false`, or explicit `null` to clear). |
| `section-type` | Section-start paragraph `target`, `value` (`continuous`, `nextPage`, `nextColumn`, `evenPage`, `oddPage`, or explicit `null`). |
| `insert-inline`, `replace-inline`, `run-bold` | Paragraph `target`, `inline_index` from `inlines`; `inline` for insert-after/replace, boolean `value` for direct run bold. |
| `table-structure` | Table `target`, `operation`. For example `{"kind":"insert-column","index":1}`. |
| `story-image` | `target` copied from `stories` (`section_index`, `kind`: `header`/`footer`, `variant`: `default`/`first`/`even`), `mode`, `bytes` as for `header-image`. |
| `style-add`, `style-replace`, `style-patch`, `style-remove` | Add/replace require `definition`. All except add require exact `id` from `styles.definitions`. Patch supports optional boolean `bold`, `italic`. |
| `core-title`, `default-alignment`, `abstract-start` | Required `value`: string, alignment name, or unsigned integer respectively. Explicit JSON `null` clears. Numbering also requires `abstract_id` and `level` from `abstract_starts`. |
| `core-subject`, `metadata`, `custom-property` | Required `value`: string or `null` (clear); custom properties also accept booleans. `metadata` takes a [field key](EDITOR-RESOURCES.md#package-metadata), and `custom-property` takes the exact `name`. |
| `control` | `target` from `controls`, optional strings `tag`, `alias`, `text`. Physical control numbers are independent of body indices, tags and XML IDs. |

`operation.kind` is `insert-row`, `remove-row`, `insert-column`, `remove-column`
(each takes `index`), `insert-cell`, `remove-cell` (each takes `row`, `cell`),
or `replace-cell` (`row`, `cell`, `blocks`). Insert before the original index
or append at the count; columns are logical, cells physical.
The `layout` operation takes a required `value`: `fixed`, `autofit`, or `null`
to clear the direct layout. `remove-table` has no other fields. `replace-node`
requires `source`; optional `row` and `cell` narrow the target to a physical
row or cell. `patch-property` takes the exact `field` name from the
[direct table property inventory](EDITOR-PARITY.md#independent-table-row-and-cell-property-edits), a required `source`
(explicit `null` clears), and optional `row`, `row` plus `cell`, or `column`
for the `width` field. Without coordinates it selects a table-level property.

A table `source` has `rows` (arrays of cell strings). It can also supply
`column_widths` in twips, `layout`, `style`, `alignment`, `look`, `borders`,
`first_row`, and `first_cell`. A donor property is taken from the first
row/cell/column where applicable, not from the addressed coordinates.
`look` accepts `first_row`, `last_row`, `first_column`, `last_column`,
`horizontal_banding`, and `vertical_banding` booleans.
`first_row` accepts `height:{"points":12,"rule":"exact"}` (`exact` or
`atLeast`), `repeat_header`, and `cant_split`. `first_cell` accepts
`grid_span`, `vertical_merge` (`restart` or `continue`), RGB `background_color`,
`vertical_alignment`, Word `text_direction` (e.g. `tbRl`), `margins` (points
for `top`, `start`, `bottom`, `end`), and `borders`. Table and cell border
records have `side`, `size` in eighth-points, RGB `color` as three bytes,
and optional `line` (`single` or `dashed`). These declarations are resolved
by the core; patching a field never imports the donor's text.

Inspection reports `layout`, `grid_widths` and direct `properties` on tables,
`layout` and `flags` on paragraphs, and `sections` with a numeric `target`,
original `body_start`, exclusive `body_end`, direct `layout`, `flags` and
`section_type_xml`.

`blocks` is an array of `{"kind":"paragraph","text":"..."}` or
`{"kind":"table","rows":[["..."]]}` (optional `column_widths`, required for
fixed-width story table payloads), or `{"kind":"rich-paragraph","inlines":[...]}`.
An `inline` is a `text` declaration
(`text`, optional boolean `bold`, `italic`), `hyperlink` (`url`, `text`), or
`image` (`bytes`). A style `definition` takes `kind` (`paragraph`, `character`
or `table`), `name`, optional `base_style` and, except for tables, optional
boolean `bold` and `italic`. These are declaration shorthands; the editor still
validates the resolved content.

`inspect` also returns `core_title`, `default_alignment`, `abstract_starts`,
`controls`, `styles`, `stories`, and `entries`. Independent failed inventory
reads return `<field>_error`. `inlines` identifies direct runs, hyperlinks and
revision wrappers with their original indices. Story and entry `blocks` list
direct paragraphs with original `block_index` and `inlines`.

`inspect` includes `core_subject`. To read specific scalar and custom fields,
pass e.g. `"metadata_fields":["manager"],"custom_properties":["Reviewed"]`
in the request. The result maps those names under `metadata` and
`custom_properties`; absent values are `null`, distinct from `""` and `false`.

## Scoped operations

Use exactly one edit per request for these operations. A physical story can
update several runs, and `part-tables` can address several eligible tables in
one owner. A separate direct-layout request can contain any combination of
`paragraph-layout` and `section-layout` records but no other kinds. As with
ordinary batches, a rejection creates no destination; inspect the saved copy
before issuing more edits.

Direct layout records take `target`, `field` (snake_case, see the
[layout field inventory](EDITOR-PARITY.md#direct-paragraph-and-section-layout)),
and required `value` (an integer, token, boolean, tab-stop array, single-border
record, or explicit `null` to clear). For example,
`{"kind":"section-layout","target":0,"field":"margin_top","value":-220}`.
Section targets come from `sections`, including table-first or empty sections.

| `kind` | Fields besides `kind` |
| --- | --- |
| `story-text` | `part` (exact physical part from `stories` or `entries`), `runs` (array of `{block_index,inline_index,text}`; also include `entry_index` for a note/comment). Aliased slots share the edit. Returns `story_part` and the two digests. |
| `story-reference` | `target` (`{section_index,kind,variant}`; can identify a non-explicit slot), `action`: `{"kind":"create","blocks":[...]}`, `{"kind":"link","part":"..."}`, or `{"kind":"remove"}`. Unlinking a slot resumes inheritance without deleting its physical part. |
| `story-blocks` | `target` from `stories`, `mode` (`shared` or `section-local`), original block `index`, `remove` (number of blocks to replace or delete), `blocks` (replacement or insertion payload). `stories[].block_count` includes tables and opaque blocks so it can be used as an append index. |
| `part-tables` | Exact `owner_part` and `operations`, an array of `{path,operation}` using `part_tables` paths from that owner. Operations are the `table-structure` forms except `replace-node` and `remove-table`. Ancestor and descendant targets cannot share one request. |
| `tracked-text` | `target` (`{target,inline_index}` for a run), `author`, optional `date`, `action` (`{"kind":"delete"}`, or `{"kind":"replace","text":"..."}` / `{"kind":"insert-after","text":"..."}`). |
| `tracked-move` | Run address objects `target`, `after`; strings `name`, `author`, `date`. |
| `review-revision` | `target` (address object for a direct revision wrapper), `action` (`accept`/`reject`), optional `paired` wrapper address for moves. |

`inspect` returns `part_tables` for the actual main part by default. To discover
physical paths in an existing header, footer, or ordinary note/comment part,
pass `"table_owner":"word/footer1.xml"` (use the exact `part` listed by
`stories` or `entries`). Paths are zero-based XML element-child coordinates
from the owner root, not body indices. The request snapshot, owner and path
together identify the table; there is no cross-owner batch.
