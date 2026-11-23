# DOCX engine audit: can rocx replace python-docx?

**Verdict:** rocx can already replace declarative generation workflows within
its documented profile. The adoption blocker is changing an existing document
without moving, losing, or inadvertently rewriting other content. Prioritize
that workflow over adding more drawing types.

## Reproduced defects

These probes used the public Python API, python-docx 1.2.0 to generate and
reopen documents, and ZIP entry comparisons for preservation. All four issues
are in the shared Rust core.

Tracking: [indexed section placement (#13)](https://github.com/ReinderVosDeWael/rocx/issues/13),
[drawing IDs (#10)](https://github.com/ReinderVosDeWael/rocx/issues/10),
[unmatched stories (#12)](https://github.com/ReinderVosDeWael/rocx/issues/12),
and [XML document-level nodes (#11)](https://github.com/ReinderVosDeWael/rocx/issues/11).

| Priority | Reproduction and observation | Fix and regression criterion |
| --- | --- | --- |
| High: wrong section membership | Create a template containing one paragraph `Original`. Render two sections containing `A` and `B` respectively with `DocumentTemplate(path, paragraph_index=0)`. Before the fix, the first section held `['A', 'B', 'Original', '']` and the second was empty. | [Insert the boundary](../src/ooxml/mod.rs#L215-L237) at the same moving position as the blocks. Assert that `A` and `B` reopen in distinct sections, in order, and verify their different margins on an indexed insertion into a nonempty template. |
| High: duplicate drawing IDs | Create a default header with one python-docx picture. Rename `word/header1.xml` and its `.rels` sibling to use `header1.part`, updating the document relationship target and content-type override. Append an image with `preview_header_image(..., mode="shared")`. Before the fix, independent readback of `wp:docPr/@id` changed from `['1']` to `['1', '1']`. | [Reserve existing drawing IDs](../src/ooxml/defaults.rs#L122) based on XML content, not the part filename. Assert that appending a picture keeps the original ID and allocates a distinct new ID. |
| Medium: a scoped search rewrites unmatched stories | Give the template a default header containing `Constant`. Use the default template scope and insert only body text. Before the fix, `word/header1.xml` differed byte-for-byte even though no edit targeted its content. | [Write a searched story](../src/ooxml/mod.rs#L119-L136) only if an edit changed it. Test a body replacement with unmatched headers and footers under the default scope; compare their ZIP entry payloads, not their projected text. |
| Medium: an alignment edit drops XML outside the document element | Add `<?example preserve?>` before the `w:document` root and `<!--tail-->` after it in `word/document.xml`. Apply one alignment edit. Before the fix, python-docx read the new alignment, but neither XML node survived. | [Preserve document-level comments and processing instructions](../src/xml/mod.rs#L256-L272) when rewriting an imported part. Test both sides of the root, in order, while allowing a new XML declaration and different serialization. |

## Replacement milestones

1. **Port a routine editing script.** Open a report, locate a paragraph by style,
   change one formatted run without losing neighboring formatting or a link,
   update a table cell, and save a copy. Today the imported paragraph projection
   flattens runs/links and the table view exposes physical cells without edit
   targets. Implement this through the existing [editor checkpoints](PLAN.md#serial-checkpoint-editor-work);
   make the script an end-to-end migration example.
2. **Exercise shared stories and real files.** Extend that script to change a
   footer for one section while preserving inherited stories elsewhere. Check
   logical cell traversal on merged tables. Supplement generated fixtures with
   redistributable Word- and LibreOffice-authored files, recording before/after
   appearance and repair prompts using the [viewer checklist](VIEWER-COMPATIBILITY-CHECKLIST.md).
   Rust/Python byte equality tests the bindings against each other, not the
   output against an independent consumer.
3. **Measure inspection costs before freezing traversal APIs.** In a probe with
   one 60-paragraph header inherited by 50 sections, Python materialized 3,000
   paragraph projections on one `default_headers` access. The [reader](../src/editor/header.rs#L126-L166)
   reparses the part per effective reference; the [Python body getter](../src/adapters/python/editor.rs#L538-L574)
   also rebuilds the whole body on each access. Benchmark opening, iterating
   and editing separately. Share immutable story content by part while keeping
   section-specific edit anchors.

Scope: targeted audit of template rendering, imported editing, package
preservation, and Python traversal. This is not a desktop viewer certification
or a full audit of every authoring codec.

## Regression evidence

Each issue had a regression that failed before its production fix and passed
afterward in Rust and Python. The Python regressions use independently
generated fixtures and the public API.

| Issue | Before the fix | Regression |
| --- | --- | --- |
| #10 | The drawing ID scan returned `0` instead of `42` for a relocated part; independent header readback returned `['1', '1']`. | [`test_relocated_drawing_ids.py`](../tests/python_bindings/test_relocated_drawing_ids.py) |
| #11 | The XML round-trip retained only the root, and the editor output was missing the leading comment. | [`test_editor_xml_envelope.py`](../tests/python_bindings/test_editor_xml_envelope.py); Rust also covers UTF-16LE and UTF-16BE. |
| #12 | The header was rewritten despite no matches; in the Rust fixture even the trailing newline was lost. | [`test_template_unmatched_stories.py`](../tests/python_bindings/test_template_unmatched_stories.py) and [`template_unmatched_stories.rs`](../tests/template_unmatched_stories.rs) |
| #13 | `A` and `B` both appeared in the first section. | [`test_indexed_template_sections.py`](../tests/python_bindings/test_indexed_template_sections.py) and [`indexed_template_sections.rs`](../tests/indexed_template_sections.rs) |

Integrated verification passed: all-target Rust tests, three doctests, Clippy,
767 schema-required Python tests, and exact Rust/Python byte parity for the
final, template, edited, and three header-scope catalog DOCX pairs.
