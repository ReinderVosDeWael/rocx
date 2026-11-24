# Issue #3 editor acceptance map

This is a qualification inventory for [#3](https://github.com/ReinderVosDeWael/rocx/issues/3),
not a new API contract. **Structural baseline: `c1b6627`.** The inline
cluster at `981d798` is accounted for separately below. In particular, the
closed scope tickets #42–#47 and #49–#51 are not claims that every declarative
operation is available in every imported context.

## Original issue requirements

| Issue requirement | What qualifies today | Limit or follow-up |
| --- | --- | --- |
| 1. Logical identities and lifetime | [Imported addresses](EDITOR-PARITY.md#existing-imported-addresses); `tests/imported_editor.rs` (`independent_body_inline_anchors_are_snapshot_scoped_and_read_only`, `story_inspection_tracks_slots_but_anchors_follow_physical_parts`, `rejects_foreign_and_duplicate_anchors_without_mutation`); catalog retry and foreign-anchor tests. CLI IDs are digest-bound addresses, not Word IDs. | Snapshot identity is supported. No durable application IDs across reopen, anchor adjustment/rebasing, or persisted resolution-to-import identity mapping. The issue expressly says not to imply persistence; decide whether this documented narrower lifetime is sufficient. |
| 2. Structured inspection | Public typed section/body/run/link/story/control/resource views. The typed catalog tests below exercise actual property and content values, not just opening or label matching. | Some modeled declarations still project as opaque or as source-property carriers. Tests for those carriers do not prove rich typed import. |
| 3. Structured editing | Immutable anchored preview, duplicate/conflict rejection, guarded replacements, reference checks and shared versus local intent. `mixed_new_edits_preserve_original_coordinates_across_parts` and `structuralResources.docx` combine node edits, a staged style, an owning relationship and numbering after a removed table. | See the operation map. Some commands are separate scoped copies, not members of the ordinary mixed batch. |
| 4. Tracked revisions | Explicit proposal and review, separate from ordinary editing; see `tracked_text_is_explicit_and_reviews_only_one_original_wrapper`, `tracked.docx`, `reviewed.docx`, `moved.docx` and `moveAccepted.docx`. | Inline cluster adds paragraph insert/delete and bold proposal/review, not arbitrary block/property contexts. |
| 5. Controls | Physical block/inline/row/cell metadata, guarded text/tag/alias updates; `controlEdits.docx` and `control_targets_refuse_active_ranges_nested_wrappers_and_ambiguous_properties`. | Inline cluster adds guarded wrappers, rich replacements and choices. Custom XML binding mutation is a separate extension per the issue, not evidence of missing current package preservation. |
| 6. Import/preservation | [Editor contract](EDITOR-PARITY.md#operation-contract), [compatibility](../COMPATIBILITY.md), `test_editor_preservation_probes.py`, independent python-docx fixtures and package-payload checks. | Schema/OPC, byte parity, typed readback, untouched payloads and viewer observations are different claims. No general lossless imported rewrite or Word rendering claim. |

## Operation groups at `c1b6627`

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
| Document and sections | Ordered section ranges, explicit break type, direct page/line numbering, page geometry, grid, equal columns, border/flags, replace supported section contents and separate anchored split/merge. Catalog `.docx`; `layout`, `sectionSplit`, `sectionMerge`; `section_lifecycle_guards_protected_gaps_and_opaque_donors`. | Unequal columns, art/theme borders and other unmodeled geometry; insert/remove of arbitrary rich sections and integrating section lifecycle with ordinary batches. |
| Stories | Six inherited/explicit header/footer slots; shared and section-local images; create/link/remove slot refs, guarded block splice and physical text; ordinary entry inlines/plain paragraph edits and base comment metadata. `.header-*`, `.footer-even`, `.story-blocks`, `.entry-*`; `storyShared`; `test_base_comment_metadata_and_block_patches_on_independent_document`. | General typed non-paragraph story blocks, editing tables in these stories, local text forks, rich entry-block replacements, deletion of physical OPC story parts and modern comment metadata. |
| Blocks and paragraphs | Direct body insert/replace/remove, property patches with omission distinct from explicit false/clear; `.edited`, `blocks`, `mixedBody`, `layout`; `patch_preserves_other_markup_and_parts_and_preview_is_non_mutating`. | General block edits and direct layout in nested/control/story paragraphs, unmodeled imported spacing/indents, and rich separate layout patches in the ordinary batch. |
| Text and inline formatting | Ordered runs, text/tabs/breaks; direct properties; guarded body insert/replace/remove, disjoint formatting patches and Unicode scalar range/point edits. `.docx`, `.run-text`; `inline`, `range`, `points`, `runFormatting`; `test_disjoint_run_format_updates_clears_and_rollbacks`. | Offsets in mixed content, other stories and across paragraphs; broader inline target contexts, themed/split font choices, and destruction of unsupported properties. |
| Tables | Direct body physical grid/spans/merges, ordered rows/cells with opaque children; insert/remove rows/cells/logical columns, replace supported cell content or table/row/cell nodes, remove table, patch selected direct properties. `.docx`, `.edited`; `tableStructure`, `tableNodes`, `structuralResources`; `test_editor_table_properties.py`. | Editing existing nested tables, story tables, controlled rows/cells as table targets and splitting protected merge chains; guarded rather than arbitrary opaque-node destruction. |
| Styles | Direct identity, kind, base/next/linked and property groups, all four style kinds, scalar and 13 conditional region edits, replace/remove unused custom definitions. `.docx`, `.run-text`; `styleEdits`; `test_imported_style_edits.py`. | Typed interpretation of every property carrier, non-scalar per-field changes and retargeting existing live users. |
| Numbering | Inspect abstract starts, concrete instances/override indexes and direct body references; add/remove abstract/instance, patch abstract start and concrete start/replacement level, change body paragraph reference. `.run-text`; `numberingEdits`, `structuralResources`; `test_imported_numbering_edits.py`. | Full typed imported abstract levels/override payloads and style links; general independent abstract-level edits, numbering reference changes in other contexts. No viewer-computed counters. |
| Defaults | Typed run/paragraph defaults and document switches, note formats, placement, start and restart; independent patch/clear. `.docx`, `.run-text`; `test_imported_defaults.py`, `test_mixed_resource_edit.py`. | Unmodeled imported default leaves or unsafe destructive replacement, rather than inferred effective formatting. |
| Metadata | Core scalars/timestamps, extended company/manager, named string and boolean custom properties; base comments as above. `.docx`, `.run-text`, `.entry-edits`; `test_mixed_resource_edit.py`, `test_resource_and_style_transport`. | Other extended values, linked or other custom types, modern comment metadata. |
| Links and ranges | Typed link children, bookmark and relationship ID; links insert/replace in eligible direct body/entry content. Catalog `.docx`, `.entry-text`; `inserted_external_hyperlink_uses_document_relationship`; owner-local ZIP checks. | Original target URL is not the raw relationship ID. Pair/tooltip/history editing and direct marker inspection require the inline cluster. Non-body pairs, rich or unsafe removal and unmodeled ranges remain after that slice. |
| Notes | Typed source kind and lexical ID, distinct part owners, simple note create/attach/detach/retarget/renumber/delete and entry content/settings. `.entry-edits` ties the new reference to its definition; `test_first_note_lifecycle_on_independent_document`. | Shared references, custom marks, nested targets, special entries and rich story lifecycle. |
| Fields | Opaque field node classification; ordinary replacement guards field boundaries. `.docx`; `whole_inline_edits_respect_cross_paragraph_fields_and_keep_opaque_formatting`. | Typed instruction/cache editing requires the inline cluster; general non-body/opaque constructs and evaluation are not implemented by it. |
| Controls and revisions | Four-context body control metadata and locked edit rejection; simple direct run revision insertion/deletion/replacement, paired moves and eligible review. `.docx`, `.run-text`; `controlEdits`, `tracked`, `reviewed`, `moved`, `moveAccepted`; `revision_inspection_keeps_boundaries_metadata_and_read_only_anchors`. | See cluster delta for typed wraps/choices, tracked paragraphs and bold. Other property and block changes, rich/nested revisions, general combined proposal and ordinary batches, and nested/bound/specialized wrapper mutation remain. |
| Images | Insert images into all six effective header/footer slots with explicit shared/local intent and relationship ownership. `.header-*`, `.footer-even`; `shared_header_edit_reports_all_consumers_and_owns_image_relationship`. Opaque body/story image readback at this baseline. | Typed occurrence/asset/geometry inspection and rich mutations require inline cluster. Even with it, not every placement, source part, opaque drawing or asset operation is supported. |
| Drawings, charts and math | Authoring and resolved fragment insertion of some objects; imported body objects remain opaque. `.docx` XML checks compare shape/chart/fraction payloads; `inserts_formatted_content_nested_table_and_fraction`. | Typed occurrence/series/fraction editing requires inline cluster, still restricted to the supported drawing/math profiles. |
| Inputs and transport | Resolved callbacks, async dependencies, refs and conditions are tested on authoring inputs, not reconstructed from DOCX. JSON CLI inspect, digest-bound preview and exclusive save-copy; `tests/cli.rs`, `test_cli_mixed.py` (including extended mixed rollback and resource/style batch). | [CLI declarations and operation set](CLI.md) are a subset; in particular rich formatting/layout, section lifecycle, story references/splices, entry lifecycle and table property/node changes are not transported in this baseline. MCP #52 is separate, not an original #3 requirement. |

## Inline cluster (`981d798`), not part of the structural baseline

Standalone evidence comes from the [plan at that commit](https://github.com/ReinderVosDeWael/rocx/blob/981d798/docs/PLAN.md),
not from a combined run with this audit. Preserve the structural capabilities and
evidence above when integrating the changes; `git diff c1b6627..981d798`
compares divergent branches, not an additive patch.

| Operation groups | Cluster evidence and scope |
| --- | --- |
| Links/ranges and fields | `fieldsRanges`, `fieldBatch`, `guardedWorkflow`; `test_imported_fields_ranges.py`; direct body paired markers and paired edits, link tooltip/history and replacement, simple/complex fields and balanced cache changes across eligible paragraphs. Comment range edits select an existing definition; creating/removing the definition is not implied. No general non-body endpoints, cached evaluation or opaque cache rewrites. |
| Inline formatting | `formatOwners` verifies edits to eligible physical cell, header/footer and note/comment runs including hyperlink children. It does not widen scalar offsets or permit arbitrary nesting. |
| Controls | `.controls`, `controlWrapped`, `controlUnwrapped`, `richControls`, `richUnwrapped`. Wrap/unwrap supported main-story inline/block/row/cell targets, guarded rich content replacement and drop-down choices. Physical owners can be inspected; other-story/bound/nested mutation is not implied. |
| Revisions | [Revision contexts at the cluster tip](https://github.com/ReinderVosDeWael/rocx/blob/981d798/docs/REVISIONS.md): proposal and review of direct bold and simple paragraph insert/delete including paragraph-mark metadata. No block moves, tracked table edits, rich revisions or general tracked property editing. |
| Images/drawings/charts/math | `.objects`, `test_editor_drawings.py` and drawing operations inside `richControls`: typed picture/shape/chart occurrence and fraction, guarded insertion/replacement/removal, selected property, fraction and single-series chart edits. Rich text-box stories, workbook/general charts and general math are outside scope. |

Add typed readback for the `.controls` and `.objects` variants in the
combined checkout. Where formerly opaque values become typed, assert the
actual properties (for example fraction operands and drawing description);
retain independently unsupported examples as opaque. The field node can
still be opaque in the flat inline view even where a dedicated field
inspector decodes it. Recheck projection/anchor semantics on the merged API,
not just that the generated bytes compare across bindings.

## Later structural integration (`c3ea09d`)

The [qualification record at `c3ea09d`](https://github.com/ReinderVosDeWael/rocx/blob/c3ea09d/docs/PLAN.md)
records a 33rd editor oracle (`physicalTables`) and the same 13 catalog
packages. Guarded nested and physical header/footer/note/comment table
property, geometry and cell-content edits now have installed-consumer
evidence, including owner-local links. Whole-node replacement/removal outside
direct body targets, controlled rows/cells, section-local table forks and
cross-owner batches still lack implementation. The CLI delta transports
direct layout, section lifecycle, story references/splices, and structural
table edits (including owner-scoped table paths); it does not establish
transport for every editor or declaration. This is separate from the inline
artifact qualification.

## Acceptance evidence and closure

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
   Chromium. Each structural catalog suffix is read back via its own binding.
   These are targeted semantic assertions (layout, formatting, story
   aliases, styles/defaults/numbering, notes, revisions, controls and opaque
   classifications), not exhaustive field-level typed import coverage.
3. **Other independent gates:** `tools/check_examples.py` and
   `tools/check_javascript.py` verify authoring coverage, exact ZIP byte
   parity, and python-docx/OOXML package readback as separate checks.
   `tools/validate_ooxml.py` covers schema and OPC relationships. Editor fixtures
   separately check unsafe edits and untouched payloads; the typed catalog
   retry checks rollback and stale anchors, not package byte preservation.

**Tracker status is not a qualification result.** GitHub already records #3
as closed; this audit does not change its state. To meet the active plan as
written, qualify the combined structural + inline artifact set from the same
revision in all installed consumers and on hosted gates; the separate branch
runs are not a substitute. Complete or explicitly rescope the
remaining modeled-operation inventory: general nested/story and rich
structure edits, richer typed style/numbering views and per-field edits,
remaining revision/control contexts, and CLI exposure of the supported editor
scope. Decide what #3 demands where the original issue says “supported” and
“incrementally” but the repository plan asks for declarative parity. Keep
durable IDs, custom XML binding, and MCP distinct from accidental closure
criteria; only a documented decision can change the currently stated plan.
