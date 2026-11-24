# Issue #3 editor acceptance map

This is the qualification inventory for the combined editor in draft
[#65](https://github.com/ReinderVosDeWael/rocx/pull/65). It records the merge of
`5be07b3` (structural and resource work) and `1fda978` (rich inlines, controls,
drawings, fields, paired ranges, revisions and their CLI transport). The owner
closed [#3](https://github.com/ReinderVosDeWael/rocx/issues/3) manually. This
document records implemented scope and evidence; it neither changes tracker
state nor claims every declarative operation works in every imported context.

## Original issue requirements

| Issue requirement | What qualifies today | Limit or follow-up |
| --- | --- | --- |
| 1. Logical identities and lifetime | [Imported addresses](EDITOR-PARITY.md#existing-imported-addresses); `tests/imported_editor.rs` (`independent_body_inline_anchors_are_snapshot_scoped_and_read_only`, `story_inspection_tracks_slots_but_anchors_follow_physical_parts`, `rejects_foreign_and_duplicate_anchors_without_mutation`); catalog retry and foreign-anchor tests. CLI IDs are digest-bound addresses, not Word IDs. | Snapshot identity is supported. No durable application IDs across reopen, anchor adjustment/rebasing, or persisted resolution-to-import identity mapping. |
| 2. Structured inspection | Public typed section/body/run/link/story/control/resource views. The typed catalog tests below exercise actual property and content values, not just opening or label matching. | Some modeled declarations still project as opaque or as source-property carriers. Tests for those carriers do not prove rich typed import. |
| 3. Structured editing | Immutable anchored preview with original coordinates, duplicate/conflict rejection and guarded replacement. `structuralResources` stages a table style and numbering across a removed table. `test_rich_cli_batch_uses_staged_resources_and_shared_reference_guards` combines paragraph replacement, wrapper, drawing, formatting, field cache, bookmark and resources; rejected requests leave no destination. | See the operation map. Scoped operations do not all participate in the ordinary mixed batch. |
| 4. Tracked revisions | Standalone text and move proposals/review, direct bold revisions, and simple paragraph insert/delete including the mark. `tracked`, `reviewed`, `moved`, `moveAccepted`, `boldChange`, `boldAccepted`, `boldRejected` and `paraInsert*`/`paraDelete*` exercise both dispositions. | No arbitrary block/property contexts or combined proposal and ordinary batches. A disjoint ordinary edit can preserve a pending revision; an edit that changes the pending paragraph mark's property envelope is refused. |
| 5. Controls | Physical block/inline/row/cell metadata, guarded wrappers, rich block/inline payload replacement, text/tag/alias updates and ordered choices. `.controls`, `controlEdits`, `controlWrapped`, `controlUnwrapped`, `richControls`, `richUnwrapped` and `guardedWorkflow`. | Story-owned controls are inspectable, not editable. Specialized, nested and bound mutation remains outside scope. |
| 6. Import/preservation | [Editor contract](EDITOR-PARITY.md#operation-contract), [compatibility](../COMPATIBILITY.md), `test_editor_preservation_probes.py`, independent python-docx fixtures and package-payload checks. | Schema/OPC, byte parity, typed readback, untouched payloads and viewer observations are different claims. No general lossless imported rewrite or Word rendering claim. |

## Combined operation inventory

Names and desired targets follow the [operation contract](EDITOR-PARITY.md#operation-contract).
Catalog filenames below omit the `all_features-<binding>` prefix. Named editor
oracles are emitted by `examples/javascript_editor_oracle.rs`, exercised by
the installed consumer in `javascript/tests/parity/integration/editor.mjs`,
and compared/independently checked by `tools/check_javascript.py` and
`javascript/tests/parity/editor.py`. Named Python tests are under
`tests/python_bindings/`; named Rust tests are in `tests/imported_editor.rs`
unless otherwise noted. Positive exemplars are not blanket coverage of all
fields or target combinations.

| Operation group | Implemented and exercised | Uncovered relative to the operation contract |
| --- | --- | --- |
| Document and sections | Ordered ranges, explicit break type, direct page/line numbering, page geometry, grid, equal columns, border/flags, replace supported section contents and anchored split/merge. `.docx`, `layout`, `sectionSplit`, `sectionMerge`. | Unequal columns, art/theme borders, arbitrary rich section replacement and unmodeled geometry. Layout requests are scoped. Section lifecycle rejects combinations with story images, control/field/range and resource operations. |
| Stories | Six inherited/explicit header/footer slots; shared and section-local images; create/link/remove slot refs, guarded block splice and physical text/tables; ordinary entry inlines/plain paragraph edits and base comment metadata. `.header-*`, `.footer-even`, `.story-blocks`, `.entry-*`; `storyShared`, `physicalTables`. | General typed non-paragraph story blocks, local text forks, rich entry-block replacements, deletion of physical OPC story parts and modern comment metadata. Use physical table inventory for nested and story tables. |
| Blocks and paragraphs | Direct body insert/replace/remove, property patches with omission distinct from explicit false/clear; `.edited`, `blocks`, `mixedBody`, `layout`; `patch_preserves_other_markup_and_parts_and_preview_is_non_mutating`. | General block edits and direct layout in nested/control/story paragraphs, unmodeled imported spacing/indents, and rich separate layout patches in the ordinary batch. |
| Text and inline formatting | Ordered runs with disjoint direct format patches; body insert/replace/remove and scalar ranges. Owner-scoped formatting also targets eligible run and hyperlink children in body cells, section stories and ordinary notes/comments. `.docx`, `.run-text`, `.entry-text`; `inline`, `range`, `points`, `runFormatting`, `formatOwners`. | Scalar offsets in mixed content, other stories and across paragraphs; arbitrary nested formatting, themed/split font choices, and destruction of unsupported properties. |
| Tables | Physical grid/spans/merges; insert/remove rows/cells/logical columns; replace supported cell content and patch direct properties. Direct body whole-node replacement/removal; physical-owner edits also cover nested and story tables. `.docx`, `.edited`; `tableStructure`, `tableNodes`, `structuralResources`, `physicalTables`. | Controlled rows/cells, protected merge splits, whole-node edits outside direct body, owner-local forks and cross-owner batches. |
| Styles | Direct identity, kind, base/next/linked and property groups; all four style kinds, direct formatting attributes and tabs, 13 conditional regions, replace/remove unused custom definitions, and guarded migration of provable same-kind references. `.docx`, `.run-text`; `styleEdits`, `styleCreate`, `styleRetarget`; `test_imported_style_fields.py`. | Typed interpretation of every property carrier, arbitrary XML edits, rewriting opaque users or field instructions, and migration of linked numbering ownership. |
| Numbering (#48) | Typed direct abstract levels, concrete overrides, stored style links and paragraph references; guarded abstract/instance/override edits, reciprocal style-owned level edits, and assignments in direct body paragraphs, eligible body cells and physical stories. `numberingEdits`, `structuralResources`, `physicalResources`, `linkedNumbering` and `linkedMixed`; `test_imported_numbering_edits.py` and `test_linked_mixed_numbering_inline_and_structure`. | No effective numbering, ownership migration, unsafe level loss, or edits to protected or arbitrary nested targets. See the [modeled resource boundary](EDITOR-RESOURCES.md#modeled-acceptance-boundary). |
| Defaults (#48) | Modeled run/paragraph defaults and document switches, note formats, placement, start and restart; independent patch/clear with no-op lexeme preservation. `.docx`, `.run-text`; `test_imported_defaults.py`, `test_mixed_resource_edit.py`. | Effective formatting and unmodeled imported default leaves or unsafe destructive replacement. |
| Metadata (#48) | Core scalars/timestamps, including `comments` as an alias for stored `description`; extended company/manager and named string/boolean custom properties. Comment entries are separate. `.docx`, `.run-text`, `.entry-edits`, `linkedNumbering`; `test_mixed_resource_edit.py`, `test_resource_and_style_transport`. | Other extended values, linked or other custom types, modern comment metadata. |
| Links and ranges | Typed link children, bookmark, relationship ID, tooltip/history; guarded link edits and direct body bookmark/comment pair insertion, relocation/removal. `.docx`, `.entry-text`, `fieldsRanges`, `fieldBatch`, `guardedWorkflow`; owner-local ZIP checks. | The relationship ID is not the URL. Non-body endpoints, creating/removing comment definitions through range edits, and unmodeled ranges are not supported. |
| Notes | Typed source kind and lexical ID, distinct part owners, simple note create/attach/detach/retarget/renumber/delete and entry content/settings. `.entry-edits` ties the new reference to its definition; `test_first_note_lifecycle_on_independent_document`. | Shared references, custom marks, nested targets, special entries and rich story lifecycle. |
| Fields | A field can be opaque in ordinary sequence inspection and still have dedicated typed instruction/flag/span readback. Guarded direct body field insertion, instruction/flag/cache patch, whole replacement/removal and balanced cross-paragraph complex-field cache edits. `.docx`, `fieldsRanges`, `fieldBatch`. | Field evaluation, opaque caches, and non-body targets. |
| Controls and revisions | See requirements 4–5. Same-block wrapper/paragraph-property conflicts and field/wrapper versus structural replacement are rejected; a staged bookmark can block revision proposals after reopening. | Nested/bound/specialized mutation, general rich tracked property edits, paragraph-mark merges with protected successors and mixed revision proposals are unsupported. |
| Images | Typed occurrence/asset/geometry inspection, selected edits for eligible body/entry pictures and explicit shared/local insertion into all six header/footer slots. `.objects`, `.header-*`, `.footer-even`, `richControls`; `test_editor_drawings.py`. | Not general shape/drawing replacement in every story; linked/opaque targets and unsupported placements still have guards. |
| Drawings, charts and math | Typed image/shape/chart occurrences and simple fraction operands. Guarded body insertion/replacement/removal, selected property patches and copy-on-write literal chart series. `.objects`; `test_editor_drawings.py`; drawing and chart `ProcessContent` probes. | General math, rich text-box stories, workbook-backed/multi-series chart edits and arbitrary opaque-node mutations. |
| Inputs and transport | Resolved authoring inputs are not reconstructed from DOCX. [CLI](CLI.md) exposes digest-bound inspect/preview/save-copy, mixed body rich and resource edits, direct layout, section lifecycle, physical tables, story splice/reference operations and scoped revision proposal/review. `tests/cli.rs`, `test_cli_mixed.py`. | CLI declarations are a subset, and physical-owner rich formatting and entry/note lifecycle have no CLI transport. MCP #52 is separate. |

## Acceptance evidence

1. **Architecture / independent imports:** `tests/fixtures/imported-editor.docx`
   was generated with python-docx. `inserts_exactly_after_anchor_in_mixed_batch_and_preserves_untouched_parts`,
   `patch_preserves_other_markup_and_parts_and_preview_is_non_mutating`,
   and `local_header_edit_restores_later_inheritors_and_preserves_link`
   test placement, unrelated markup and untouched payloads. The packed
   consumer compares combined `mixed` and `mixedShared` outputs to Rust; its
   independent reader checks the actual headers and relationships.
2. **Typed catalog readback:** `tests/showcase.rs` and
   `tests/support/catalog_readback.rs` assert the Rust artifacts;
   `test_all_features.py` and `catalog_readback.py` assert the generated
   installed Python artifacts; `catalog-contract.mjs` and
   `catalog-readback.mjs` assert the generated artifacts in installed Node and
   Chromium. The same 15 catalog suffixes have typed readback in all three
   bindings. In the new checks, `.controls` asserts the selected value and
   display label, block plain-text kind, explicit `multiLine=0`, and
   foreign-anchor rejection. `.objects` asserts picture description and
   relationship ID, ellipse geometry and text, chart labels/values, fraction
   operands, the sibling wrapper context and both footnote image relationship
   IDs across physical paragraphs. `.entry-text` checks 13.5-point linked
   runs in each ordinary entry; `.docx` checks the dedicated PAGE field
   view alongside the opaque inline node and the typed fraction operands.
   These are selected properties, not exhaustive field-level import coverage.
3. **Other independent gates:** `tools/check_examples.py` and
   `tools/check_javascript.py` verify authoring coverage, exact ZIP byte
   parity, and python-docx/OOXML package readback as separate checks.
   `tools/validate_ooxml.py` covers schema and OPC relationships. Editor fixtures
   separately check unsafe edits and untouched payloads; the typed catalog
   retry checks rollback and stale anchors, not package byte preservation.

4. **Mixed conflicts and preservation:** the rich CLI fixture checks exact
   installed-Python bytes, stale/unsafe target checks elsewhere in the CLI
   suite, and unchanged ZIP parts. It rejects a structural replacement that
   overlaps a staged wrapper or field cache, a section boundary request mixed
   with staged rich operations, and destructive replacement of a pending
   paragraph revision. The disjoint structural replacement succeeds in the
   original batch and the pending revision remains reviewable after the failed
   replacement. `test_destructive_resources_recheck_new_rich_content` proves
   finished-package style/numbering reference checks; matched
   `ProcessContent` wrappers in Word and DrawingML are rejected separately
   from unrelated opaque extensions.

5. **#48 merged follow-up:** `linkedMixed` edits a reciprocal style-owned
   level and a run, removes an earlier table, and applies numbering to a
   retained cell using its original anchor. It preserves the alias, concrete
   overrides and untouched package parts. CLI and binding tests reject
   overlapping cell/table edits and removal of the owning instance; the
   binding tests also reject stale anchors. The independent follow-up
   qualification, including both packed runtimes, is recorded in [the plan](PLAN.md).

On `272e3c6`, local packed Node 22/24 and Chromium 159 development/production
checked 50 editor outputs and 15 catalog packages against exact Rust bytes.
The installed Python catalog matched the 15 artifacts as well. Type checking,
offline package replay, independent readback, schema/OPC and untouched
payload verification passed as distinct gates. The
[active plan](PLAN.md) records the native, wheel and docs runs.

**Tracker status is not a qualification result.** #3 is closed and #65 remains
draft. The unsupported cases above are scope limits. Hosted qualification
on the combined head is separate.
