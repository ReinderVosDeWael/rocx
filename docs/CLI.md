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
| `paragraph-flag`, `section-flag` | Paragraph `target` (section start for section flags), `flag` (Python snake_case name), `value` (`true`, `false`, or explicit `null` to clear). |
| `section-type` | Section-start paragraph `target`, `value` (`continuous`, `nextPage`, `nextColumn`, `evenPage`, `oddPage`, or explicit `null`). |
| `insert-inline`, `replace-inline`, `run-bold` | Paragraph `target`, `inline_index` from `inlines`; `inline` for insert-after/replace, boolean `value` for direct run bold. |
| `inline-edit` | Paragraph `target`, `inline_index`, and the rich inline `operation` below. |
| `field` | Paragraph `target`, original `inline_index` of a simple field or complex field begin run, and `operation`. |
| `paired-range` | `id`, optional `bookmark`, and optional pair `endpoints` of inclusive `{target,inline_index}` addresses. No bookmark name means an existing comment definition; no endpoints removes the pair. |
| `table-structure` | Table `target`, `operation`. For example `{"kind":"insert-column","index":1}`. |
| `story-image` | `target` copied from `stories` (`section_index`, `kind`: `header`/`footer`, `variant`: `default`/`first`/`even`), `mode`, `bytes` as for `header-image`. |
| `style-add`, `style-replace`, `style-patch`, `style-remove` | Add/replace require `definition`. All except add require exact `id` from `styles.definitions`. Patch supports optional boolean `bold`, `italic`. |
| `style-field` | Style `id`, `field`, optional `region`, and `value` (string or explicit `null`). Uses the [direct style field keys](EDITOR-PARITY.md#imported-style-edits). |
| `style-region` | Style `id`, `region`, and `value` (format object or explicit `null` to clear). |
| `core-title`, `default-alignment`, `abstract-start` | Required `value`: string, alignment name, or unsigned integer respectively. Explicit JSON `null` clears. Numbering also requires `abstract_id` and `level` from `abstract_starts`. |
| `core-subject`, `metadata`, `custom-property` | Required `value`: string or `null` (clear); custom properties also accept booleans. `metadata` takes a [field key](EDITOR-RESOURCES.md#package-metadata), and `custom-property` takes the exact `name`. |
| `document-default` | `field` from the [defaults inventory](EDITOR-RESOURCES.md#editing-document-defaults), `value` in its documented scalar type or explicit `null`. The `tabStops` value is an array of objects with `position_twips`, `alignment`, and optional `leader`. |
| `add-abstract`, `remove-abstract` | `abstract_id`; addition requires `levels` (numbering level objects). |
| `add-instance`, `remove-instance` | `num_id`; addition requires `abstract_id`. |
| `instance-start`, `instance-level` | `num_id`, `level`, and `value` (unsigned integer or numbering level object, respectively; explicit `null` clears). |
| `paragraph-numbering` | Paragraph `target` and `value` (`{"id":42,"level":0}` or explicit `null`). |
| `control` | `target` from `controls`, optional strings `tag`, `alias`, `text`. Physical control numbers are independent of body indices, tags and XML IDs. |
| `control-wrapper` | An `operation` using `controls` or `control_sites` indices, as described below. |

`operation.kind` is `insert-row`, `remove-row`, `insert-column`, `remove-column`
(each takes `index`), `insert-cell`, `remove-cell` (each takes `row`, `cell`),
or `replace-cell` (`row`, `cell`, `blocks`). Insert before the original index
or append at the count; columns are logical, cells physical.
The `layout` operation takes a required `value`: `fixed`, `autofit`, or `null`
to clear the direct layout. Inspection includes `layout` and `grid_widths`
on tables, `flags` on paragraphs, and `sections` with original `body_start`,
exclusive `body_end`, direct `flags`, and `section_type_xml`.

`blocks` is an array of `{"kind":"paragraph","text":"..."}` or
`{"kind":"table","rows":[["..."]]}`. Supported `inline` declarations are:

| `kind` | Fields besides `kind` |
| --- | --- |
| `text` | `text`; optional `bold`, `italic`, `style` (character style name or ID), and `size` (points). |
| `hyperlink`, `bookmark-link` | `url` or `name`, respectively, and `text`. A bookmark link can target a pair staged in the same batch. |
| `image` | `bytes`, as for `header-image`. |
| `fraction` | `numerator`, `denominator`. |
| `column-chart` | `width`, `height` in points; `categories` (strings), `values` (numbers). |
| `field` | `instruction` object (`page`, `num-pages`, `table-of-contents`, `reference` with `name`, or `raw` with `text`); optional `children` (cached inlines), `complex`, `dirty`, `locked`. |

An `inline-edit` operation is `format` (`set` with optional `style`, `font`,
`size`, `bold`, `italic`, `strike`, `all_caps`, `small_caps`; `clear` is a list
of core format field keys such as `size` or `fontEastAsia`), `drawing-description`
(`value`: string or `null`), `drawing-property` (`property` such as `size` or
`chartSeries`, plus an `inline` declaration of the appropriate drawing type),
`image-asset` (`bytes`), `remove-object`, `hyperlink-tooltip`, `hyperlink-history`
(`value` or explicit `null`), or `remove-hyperlink`.

A `field` edit operation is `instruction` (`value`: string), `dirty` or
`locked` (`value`: boolean or `null`), `cache` (`children`), `replace` (`inline`
field declaration), or `remove`. These edit kinds do not evaluate fields.

A `control-wrapper` operation is `wrap` (`first` and optional `last` from
`control_sites`, `tag`), `unwrap` (`target` from `controls`), `replace-inlines`
(`target`, `inlines`), `replace-blocks` (`target`, `blocks`), `choice`
(`target`, `selected`, optional `items` of `{display_text,value}`), or `define`
(`target`, `text` object, optional `id`, `tag`, `alias`, `lock`). The definition
`text.kind` is `rich`, `plain` (optional `multi_line`), or `drop-down` (`items`,
`selected`); lock names are `unlocked`, `control`, `content`, or `both`.

Paragraph, character and table style definitions take
`kind`, `name`, optional `base_style` and, except for tables, optional boolean
`bold` and `italic`. A `numbering` definition takes `name` and `numbering_id`.
A numbering level takes `level`, `start`, `format`, `text`, and optionally
`is_legal` and `restart`. A `style-region` format object accepts `bold`, `italic`,
`underline`, `font`, `font_size`, `color`, `background`, `alignment`,
`spacing_before`, `spacing_after`, and `bidi`. Colors are arrays of three RGB
bytes. These are declaration shorthands; the editor validates the resolved
content and references.

`inspect` also returns `core_title`, `default_alignment`, `abstract_starts`,
`controls`, `control_sites`, `styles`, `stories`, and `entries`. Independent
failed inventory reads return `<field>_error`. `inlines` identifies direct runs,
simple fields, fractions, hyperlinks and revision wrappers with original
indices. Runs can carry `drawing`, `has_field_marker` or `property_revision`;
paragraphs with a tracked terminator report `mark_revision`. Story and entry `blocks` list
direct paragraphs with original `block_index` and `inlines`.

`inspect` includes `core_subject`. To read specific scalar and custom fields,
pass e.g. `"metadata_fields":["manager"],"custom_properties":["Reviewed"]`
in the request. The result maps those names under `metadata` and
`custom_properties`; absent values are `null`, distinct from `""` and `false`.
It also returns `document_defaults` and `numbering_instances` (including direct
start overrides and replacement level indices). To read specific paragraph
references, pass `"numbering_targets":[0,2]`; `paragraph_numbering` maps those
original body indices to direct references or `null`. Style definitions include
direct `next_style`, `linked_style`, and `property_xml`.

## Scoped operations

Use exactly one edit per request for these operations. A physical story can
update several runs in that edit. As with ordinary batches, a rejection
creates no destination; inspect the saved copy before issuing more edits.

| `kind` | Fields besides `kind` |
| --- | --- |
| `story-text` | `part` (exact physical part from `stories` or `entries`), `runs` (array of `{block_index,inline_index,text}`; also include `entry_index` for a note/comment). Aliased slots share the edit. Returns `story_part` and the two digests. |
| `tracked-text` | `target` (`{target,inline_index}` for a run), `author`, optional `date`, `action` (`{"kind":"delete"}`, or `{"kind":"replace","text":"..."}` / `{"kind":"insert-after","text":"..."}`). |
| `tracked-move` | Run address objects `target`, `after`; strings `name`, `author`, `date`. |
| `tracked-bold` | Run address object `target`, boolean `value`, `author`, optional `date`. |
| `tracked-paragraph` | Paragraph `target`, `author`, optional `date`, and `operation` (`{"kind":"delete"}` or `{"kind":"insert-before","text":"..."}`). |
| `review-revision` | `target` (address object for a direct revision wrapper or run property revision), `action` (`accept`/`reject`), optional `paired` wrapper address for moves. |
| `review-paragraph-revision` | Paragraph `target`, `action` (`accept`/`reject`); reviews the paired terminator and text together. |
