# DOCX compatibility

rocx writes **Transitional DOCX**. It does not paginate, render pages or calculate fields, counters or effective styles. See [viewer compatibility](docs/VIEWER-COMPATIBILITY-CHECKLIST.md) for LibreOffice results. Desktop Word has not been verified.

## Implemented surface

| Area | Supported |
| --- | --- |
| Text and layout | Unicode, whitespace, empty runs, tabs, breaks, run/paragraph formatting, columns, page settings and default/first/even headers and footers. [Formatting](docs/FORMATTING.md) |
| Resources | Document defaults, paragraph/character/table/numbering styles, list levels and concrete overrides. [Styles](docs/STYLES.md) · [Numbering](docs/NUMBERING.md) |
| Tables | Nested tables, fixed/autofit widths, spans, vertical merges, borders, shading, alignment, row pagination requests and conditional regions. [Tables](docs/TABLES.md) |
| References | Links, overlapping bookmarks and body-comment ranges, structured comments, footnotes, endnotes, simple and complex fields with cached results. [Ranges](docs/RANGES.md) · [Notes](docs/NOTES.md) · [Fields](docs/FIELDS.md) |
| Drawings | Inline or anchored PNG/JPEG/GIF/BMP/TIFF with density-aware dimensions and alt text; rectangles, ellipses, plain text boxes, single-series column charts and literal inline fractions. [Drawings and restrictions](docs/FORMATTING.md#picture-geometry-and-floating-placement) |
| Review and forms | Tracked text insertions, deletions and paired moves; inline/block controls and row/cell wrappers; tagged plain-text replacement. [Revisions](docs/REVISIONS.md) · [Content controls](docs/CONTENT_CONTROLS.md) |
| Metadata | Core properties, snapshot timestamp, company, manager and [custom document properties](#document-properties). |

Rust and Python examples use their respective public APIs. For the available JavaScript factories, templates, inspection and edit previews, see the [Node/browser package guide](javascript/README.md). Preserving an imported feature does not mean it is editable.

**Hyperlinks:** use absolute URI/IRI targets for external links and explicit bookmark targets for internal ones. External targets preserve Unicode, escapes, queries and fragments; invalid percent escapes, whitespace, controls and forbidden URI characters fail. This checks syntax, not safety. rocx never fetches targets. Format display runs explicitly; links are not automatically blue or underlined. Nested links are unsupported.

## Units and defaults

| Setting | Unit |
| --- | --- |
| Run size, paragraph spacing/direct indents, image dimensions | Points |
| Paragraph **style** left/right/first-line indent | Inches |
| Section page sizes and margins | EMUs (914,400/inch) |
| Table widths and column spacing | Twips (1,440/inch) |
| Border size | Eighth-points |
| `line_spacing` | Multiplier |

Rust provides checked `Points`, `Twips`, `Emu` and `HalfPoints` conversions. The blank template uses an 8.5 × 11 inch page with 1.25 inch left/right margins.

## Document properties

```python
from rocx import declarative as d

document = d.Document([], title="Annual report", company="Atlas", manager="Ada",
                      custom_properties={"Project": "Atlas", "Approved": False})
```

In Rust, use `Metadata::with_company`, `with_manager`, `with_custom_properties` (strings) and `with_custom_boolean_properties`. Custom names must be nonblank and match exactly. Omit a property to keep its template value; `""` sets empty text rather than deleting it. Updating a linked, ambiguous or differently typed custom property fails.

## Resolution, editing and output

| Imported DOCX operation | Rust | Python | JavaScript |
| --- | --- | --- | --- |
| Combine all supported edit kinds atomically | `doc.preview_mixed(...)` | `doc.preview(alignments, ...)` | `doc.preview(batch)` |
| Inspect footnotes, endnotes and comments | `doc.entry_stories()?` | `doc.entry_stories` | `doc.entryStories()` |
| Inspect the styles part and direct style metadata | `doc.styles()?` | `doc.styles` | `doc.styles()` |
| Inspect direct body control metadata | `BodyBlock::ContentControl` | `BodyContentControl` | `kind: "contentControl"` |
| Inspect an inline insertion/deletion | `ImportedInlineNode::Revision` | `ImportedRevision` | `kind: "revision"` |
| Track a direct body text edit / paired move | `preview_tracked_text` / `preview_tracked_move` | `preview_tracked_text` / `preview_tracked_move` | `previewTrackedText` / `previewTrackedMove` |
| Accept/reject supported anchored revision(s) | `preview_revision` | `preview_revision` | `previewRevision` |
| Insert a paragraph after a direct body paragraph | `preview_with_paragraphs` / `ParagraphInsertion` | `preview([], paragraph_insertions=[edit])` | `preview({paragraphInsertions: [edit]})` |
| Set a direct body table cell's vertical alignment | `preview_with_cells` / `CellAlignmentEdit` | `preview([], cell_alignments=[edit])` | `preview({cellAlignments: [edit]})` |
| Replace text in a direct body run | `doc.preview_with_text(&[], &[], &[], &[edit])?` | `doc.preview([], run_text=[edit])` | `doc.preview({runText: [edit]})` |
| Remove a direct body run | `doc.preview_with_run_removals(&[], &[], &[], &[], &[anchor])?` | `doc.preview([], run_removals=[anchor])` | `doc.preview({runRemovals: [anchor]})` |
| Inspect removed runs | `preview.run_removals()` | `preview.run_removals` | `preview.runRemovals()` |
| Inspect changed runs | `preview.run_text_changes()` | `preview.run_text_changes` | `preview.runTextChanges()` |

Entries include the original ID, note type or comment
metadata, and direct paragraphs with snapshot-scoped read-only inline anchors.
Tables and other block types are opaque in these stories.

Use `RunTextEdit { target, text }` in Rust, `RunTextEdit(target, text)` in Python,
or `{target, text}` in JavaScript. The run must
contain a single text element outside fields and ranges. Formatting, adjacent
runs and opaque siblings survive. Tabs, line breaks, mixed-content runs,
duplicate targets and stale anchors reject the entire batch.
Removal deletes the whole selected run, not just its text. It accepts only
single-text direct body runs with recognized local formatting; unknown attributes,
complex properties, revisions, mixed content and protected ranges are rejected.
Replacement and removal of the same anchor conflict.

- Rust consumes declarations; Python leaves them unchanged. [Resolve once](docs/DEFERRED-FIELDS.md) to render repeatedly without rerunning producers or rereading images.
- **Compose** with an unsigned Transitional DOCX or DOTX template; output is DOCX. **Inspect and preview edits** on an unsigned DOCX: align direct body paragraphs, insert a supported paragraph or table, patch physical cell alignment, edit supported body runs, or append an inline image to an existing default or first-page header. A shared edit affects all aliases of that physical header; a section-local edit forks a supported header for the selected slot. Use targets from the same snapshot. See the [editor inventory](docs/EDITOR-PARITY.md) for target and preservation limits.
- [Template replacement](https://reindervosdewael.github.io/rocx/recipes/#edit-an-existing-document) can span runs within a paragraph or hyperlink. It cannot cross structured boundaries or change fields, revisions or controls. Replacement text belongs to the first matched run; bookmark/comment markers and untouched break attributes stay in place.
- Untouched **part payloads** retain their bytes. Edited XML and the ZIP container may be reserialized. Related parts are found by relationship, not filename; invalid internal relationships fail. Path output replaces the destination only after compilation, using a sibling temporary file.
- Packages declaring a digital-signature origin are rejected, even without edits. Use an unsigned copy; rocx does not strip, verify or renew signatures.
- On edited parts, unsupported or unbound `mc:MustUnderstand` namespaces on WordprocessingML elements are rejected. The guard recognizes Transitional WordprocessingML, Office relationships and XML. It does not select `AlternateContent` branches or scan foreign subtrees and untouched opaque parts.
