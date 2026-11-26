# Full MCP parity with the Rust document editor

Track delivery in [#72](https://github.com/ReinderVosDeWael/rocx/issues/72).
Baseline: `b38ac22` in [PR #71](https://github.com/ReinderVosDeWael/rocx/pull/71).
[#52](https://github.com/ReinderVosDeWael/rocx/issues/52) qualifies the initial
six tools and seven edits; it need not wait for this follow-up.

## Implementation checkpoint

The [parity inventory](../tests/mcp_parity/README.md) starts at the three
compiled editor roots. Its first snapshot has 70 `ImportedDocument` methods,
18 `EditBatch` fields and 10 `EditPreview` methods. Separate entries record
reachable payload types, fields and variants; the symbol count is **not** a
measure of user-facing feature completion.

The first ordinary-batch extension transports `TextRangeEdit` and
`TextInsertion` as [paragraph-anchored scalar edits](MCP.md#edit-part-of-a-paragraph).
The installed-client fixture covers non-BMP text, mixed formatting, runless
paragraphs, rejection/retry and equality with the native saved package.
The next inspection slice adds a paged `entries` view and an owner-scoped
`edit-entry` operation for the native `EntryEdit` variants. An entry page
projects block text and opaque markers; it does not expose the full typed
inline tree. The scoped `edit-note-reference` operation is separate from
entry definitions; the note body-site view advertises candidate insertion
runs and original lexical reference IDs. Relocated note parts with zero
entries cannot yet be identified through the public entry catalog, and
`edit_entry_inlines` remains open.

The same bounded paging model now supports direct paragraph/section
`layout` and pending `revisions`. Scoped previews delegate respectively to
native layout transactions and the six revision proposals/review methods.
Open supports per-request `ReadLimits`, and `no-op` stages the native empty
batch without changing the established nonempty `edits` request.

## What counts as parity?

Start from the public `rocx::editor` API, especially `ImportedDocument`,
`EditBatch`, `EditPreview`, and the types reachable from their public
signatures. A client must be able to discover a supported target, read the
same modeled values, supply the same edit **including optional fields**, and
obtain the same report and DOCX as a direct Rust caller. This covers physical
owners and scoped methods as well as the ordinary body batch. The
[Rust acceptance map](EDITOR-ACCEPTANCE.md) supplies fixtures and domain
boundaries; the [CLI](CLI.md) is a reuse reference, not the inventory.

Multiple Rust convenience methods may map to one tool request if semantics
and results coincide. Record that mapping. Represent resolved paragraph,
inline, table and other payloads accepted by the editor, including nested
fields. Keep the existing plain-paragraph `create_document`; implementing a
general authoring language, running Rust callbacks, rendering Word, or
inventing edits over opaque XML is outside this editor milestone. If a
supported editor value lacks a portable representation, it is a **gap**
until represented or explicitly adjudicated on #72.

## Wire and implementation decisions

- Preserve the current six calls and their existing valid inputs, outputs,
  handles and errors. Unqualified `inspect_document` keeps its compact view.
  Add a typed `view` selector with domain-specific selectors, owner anchors and
  bounded pages for large catalogs. Page cursors belong to the document, view
  and query. Return explicit units, indices, direct vs inherited state and
  opaque/read-only indicators. Clients must never need to construct an anchor
  from a path, XML name or numeric offset. For positions, inspect the parent
  and expose the native coordinate system (Unicode scalar, block gap,
  physical cell vs logical column, etc.).
- Extend `preview_edits`. Keep `{document, edits:[...]}` for ordinary mixed
  edits. Add a mutually exclusive `{document, operation:{kind:..., ...}}`
  for separately scoped calls. A scoped operation contains an owner and a
  list only where the Rust method itself accepts a list. Reject ambiguous
  requests. Never simulate atomicity by sequentially editing/reopening ZIPs.
  For native methods returning bytes rather than `EditPreview`, register
  those exact bytes and expose only change data the core actually provides.
- Issue opaque, typed handles backed by original core identities. Distinguish
  logical section slots from shared physical parts, reference sites from
  entry definitions, before/after positions from nodes, and resource IDs
  from instance handles. Repeated queries must agree on identity; lazy
  inspection and pagination must not invalidate earlier anchors. Source and
  preview handles, exclusive save and close retain the [lifetime contract](MCP-PLAN.md).
- Derive discovery schemas and runtime decoding from the same types. Split
  schemas and conversion code by domain under `crates/rocx-mcp`; the session
  owns lifetime and files, the Rust core owns validation and mutation. For
  patches, encode keep (omitted), set (including false/zero/empty where
  supported), and clear distinctly. Specify exact wire encodings for ID
  lexemes, timestamps, bytes, units and integers beyond JSON's safe range.
  Transport supplied values and embedded data, not host-side callbacks.
  Inventory import limits (`open_with_limits`), no-op previews, all report
  accessors and error paths alongside visible edits; preserve existing server
  capacity limits and document how per-open limits interact with them.

## Delivery slices

Land the baseline first. Each numbered slice is independently reviewable:
ship its discovery, input conversion, readback, installed-client tests and
guide examples together. Read-only views can land before mutations.

| Slice | Deliverable | Required proof beyond a happy-path call |
| --- | --- | --- |
| 0. Freeze the inventory | At the implementation base, enumerate public methods (including macro-generated methods), all 18 `EditBatch` slices in `src/editor/batch.rs`, reachable enum variants, struct fields, owner contexts, result fields and convenience aliases. Create a versioned machine-readable parity manifest and a CI drift check against the compiled public surface. Seed it with the seven working variants and their narrower payloads/owners marked **partial** where appropriate. | Each row has Rust symbol, scope, target/view, MCP spelling, status, positive and negative fixture IDs. Demonstrate that adding an unmapped Rust field or variant fails CI. Reconcile against current source again at closure. |
| 1. Inspection and addressing | Add views for sections/blocks, inlines, hyperlink children, scalar spans, gaps, tables, all six header/footer slots, ordinary entry stories, revisions, controls, fields, paired markers, drawing occurrences/chart series, styles, numbering, defaults and named metadata. Add import options and report mapping. | Compare typed values to direct Rust inspection (including absent, false, opaque and mixed content), not XML summaries. Exercise shared parts, empty sections, paged catalogs, invalid cursor reuse and foreign handles. |
| 2. Body structure and layout | Implement scalar point/range edits, rich paragraph/table insertion, block/section content replacement and removal, paragraph/section flags, direct layout, section split/merge. Add resolved declaration transport incrementally; cover the full editor-accepted model, not only text shorthands. | Mix an insertion with an original-coordinate replacement; test non-BMP offsets, empty final sections, overlap, destructive opaque-content rejection and retry. Preserve `preview_layout` scope. |
| 3. Inline content, fields and ranges | Implement supported inline insert/replace, direct run formatting, link properties and targets, field insertion/replacement/flags/cache, bookmark and comment pairs; cover physical-owner `edit_run_format`. | `formatOwners`, `fieldsRanges` and `fieldBatch`: linked runs in physical owners, balanced cross-paragraph field cache, disjoint patches vs crossing/protected ranges. Verify relationship targets independently. |
| 4. Tables and stories | Body, nested and physical tables: properties, rows/cells/logical columns, cell content and guarded whole nodes. All six story image slots; create/link/remove explicit references, splice blocks, physical text; note/comment entry edits and note reference/definition lifecycle. | `physicalTables`, `storyShared` and first-note lifecycle: grid/merge guards, inherited slot vs physical owner, shared vs local fork, relocated parts, reserved entries, stale references and wrong-owner batching. |
| 5. Resources | Metadata and defaults, all modeled style kinds/fields/conditional regions, typed numbering levels, abstract/instance/override edits, direct paragraph assignments and guarded style retargeting. Include style/numbering changes in ordinary mixed batches. | `linkedMixed` plus absent-part creation: assign a staged resource to an originally addressed cell after earlier table removal; verify lexical no-ops, reciprocal style links and rejection of live/opaque references without a partial edit. |
| 6. Rich objects and controls | Picture/shape/chart/fraction inspection and supported edits, including eligible entry pictures and copy-on-write chart data. Discover block/inline/row/cell control sites and implement supported wrap, unwrap, content/definition replacement, properties and choices. | `richControls`, `controlWrapped` and `.objects`: inspect operands and series, verify relationship ownership and untouched media; reject protected/nested/bound sites and incompatible field/control replacements. |
| 7. Tracked revisions | Proposal and review of supported text insertion/replacement/deletion, moves, bold changes and paragraph revisions including marks. Expose revision metadata and both accept/reject dispositions. | Independently inspect proposed and reviewed packages; verify paired move IDs, protected successors, original-anchor rejection after reopen, and that ordinary edits cannot be smuggled into a revision request. |
| 8. Close the matrix | Reconcile every Rust surface row, including error-only cases, declaration leaves, reports and convenience aliases. Replay combined fixtures and document domain limits and supported request forms. | Zero unexplained gaps; all required installed-client gates pass at the same commit on `main`. Link manifest, user guide, CI run and remaining **core** limitations on #72. |

Slices 2–7 depend on the manifest and the relevant addressing work in slice 1.
Build body declaration conversion once in slice 2; reuse it for table/story
content, fields and controls. Owner-specific formatting and drawings use
slice 4's physical identities. Finish the mixed resource oracle after both
body and physical table support. No domain is “complete” while a required
owner or input field is still delegated to a later slice.

## Qualification and drift control

1. Extend `tests/mcp/` to launch the **installed** binary with the independent
   MCP client, negotiate stdio and validate the actual advertised schemas.
   The parity manifest should drive a coverage check that also verifies the
   referenced tests exist. Validate schema size/discovery and bounded
   inspection with a large document in OpenCode; use a real OpenCode smoke
   call per slice in addition to SDK CI.
2. Reuse deterministic fixtures in `examples/javascript_editor_oracle.rs`,
   `tests/imported_editor.rs` and the [existing acceptance inventory](EDITOR-ACCEPTANCE.md).
   Extend `crates/rocx-mcp/examples/mcp_byte_oracle.rs` or reuse a direct Rust
   oracle, never the MCP converter, as reference. Use identical inputs,
   timestamps and operations; compare **whole saved ZIP bytes**. Compare
   typed inspection, full reported changes and errors separately, normalizing
   only session-specific handle identities.
3. Generate independent fixtures with `python-docx`. Reopen successful outputs
   with it; check XML/ZIP separately for opaque markup, MCE, media,
   relationships, unchanged part payloads and schema/OPC validity. Include
   Word features `python-docx` cannot model using existing package fixtures.
4. Per transaction family, cover rejection and retry, foreign/stale handles,
   illegal scope combinations, semantic no-op, alternating previews and exact
   selected-preview save. Re-run the six-tool baseline on Rust 1.88 and stable.
   If a slice actually changes a public core capability, first qualify it in
   Rust, Python and JavaScript with declarations, catalogs and parity checks.
   Transport-only work must not imply new core or CLI support.
