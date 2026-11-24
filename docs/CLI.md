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

The result includes the source `snapshot` and seven arrays of actual changes:

- `changes`: legacy paragraph alignment records (`target`, `before`, `after`;
  `after` retains Rust spelling, e.g. `Center`).
- `paragraph_insertions`: original paragraph target numbers.
- `table_insertions`: `target`, `rows`, `columns`.
- `run_text` / `run_removals`: `target`, `inline_index`, `before`; replacements
  also include `after`.
- `cell_alignments`: `target`, `row_index`, `cell_index`, `before`, `after`.
- `header_images`: `mode`, `source_part`, `output_part`, `affected_sections`,
  `restored_section` (nullable).

Each list follows the core's original document/section order; unchanged values
are omitted. `before` on an alignment is the original direct XML value or `null`.

## Snapshot and extension contract

`snapshot` is lowercase SHA-256 of the complete input DOCX bytes. Every preview
or save reads the source once, checks the digest, then binds addresses against
that same byte buffer in a newly opened core snapshot. Identical bytes at another
path are interchangeable. Any byte change, even a ZIP metadata change, invalidates
the digest. This is change detection, not authentication or a lock on a path.

The CLI is a transport for `ImportedDocument::preview_mixed`. Paragraph, table
and image shorthand constructs ordinary declarations and uses `Component::resolve`;
it does not serialize `Resolved*` objects or accept arbitrary OOXML. Richer
declaration forms can get explicit, strictly decoded fields or new tagged
variants. For a future core edit kind, expose any new address in `inspect`, bind
it against the same open document, and map the core's change record into the
report. Preserve the existing spellings and test the new form alongside mixed
batches, stale digests and byte-equivalent binding output.
