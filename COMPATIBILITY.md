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
| Combine ordinary edits atomically ([all batch fields](docs/EDITOR-PARITY.md#combine-ordinary-edits)) | `doc.preview_edits(EditBatch { .. })` | `doc.preview(alignments, ...)` | `doc.preview(batch)` |
| Inspect footnotes, endnotes and comments | `doc.entry_stories()?` | `doc.entry_stories` | `doc.entryStories()` |
| Inspect the styles part and direct style metadata | `doc.styles()?` | `doc.styles` | `doc.styles()` |
| Inspect direct body control metadata | `BodyBlock::ContentControl` | `BodyContentControl` | `kind: "contentControl"` |
| Inspect block/inline/row/cell controls and their physical anchors | `doc.controls()` | `doc.controls` | `doc.controls()` |
| Patch an eligible control's alias, tag or single-text value | `ControlEdit` / `preview_with_controls` | `ControlEdit` / `preview([], controls=[edit])` | `preview({controls: [edit]})` |
| Inspect an inline insertion/deletion | `ImportedInlineNode::Revision` | `ImportedRevision` | `kind: "revision"` |
| Track a direct body text edit / paired move | `preview_tracked_text` / `preview_tracked_move` | `preview_tracked_text` / `preview_tracked_move` | `previewTrackedText` / `previewTrackedMove` |
| Accept/reject supported anchored revision(s) | `preview_revision` | `preview_revision` | `previewRevision` |
| Insert a paragraph after a direct body paragraph | `preview_with_paragraphs` / `ParagraphInsertion` | `preview([], paragraph_insertions=[edit])` | `preview({paragraphInsertions: [edit]})` |
| Replace/remove a paragraph, replace section content or patch keep-with-next | `BlockEdit` / `preview_blocks` | `preview([], block_edits=[edit])` | `preview({blockEdits: [edit]})` |
| Insert/replace a direct inline or patch run bold | `InlineEdit` / `preview_with_inlines` | `preview([], inline_edits=[edit])` | `preview({inlineEdits: [edit]})` |
| Insert/remove rows, cells or columns; replace cell contents | `TableStructureEdit` / `preview_with_table_structure` | `preview([], table_structure=[edit])` | `preview({tableStructure: [edit]})` |
| Edit supported style definitions | `StyleEdit` / `preview_with_styles` | `preview([], style_edits=[edit])` | `preview({styleEdits: [edit]})` |
| Set/clear title, default alignment or abstract level start | `ResourceEdit` / `preview_with_resources` | `preview([], resources=[edit])` | `preview({resources: [edit]})` |
| Edit simple text in a physical header/footer, footnote/endnote or comment part | `edit_story_text(owner_part, edits)` | `edit_story_text(owner_part, edits)` | `editStoryText({ownerPart, runText})` |
| Insert/replace an ordinary note/comment inline or patch run bold | `edit_entry_inlines(owner_part, edits)` | `edit_entry_inlines(owner_part, edits)` | `editEntryInlines({ownerPart, inlineEdits})` |
| Set a direct body table cell's vertical alignment | `preview_with_cells` / `CellAlignmentEdit` | `preview([], cell_alignments=[edit])` | `preview({cellAlignments: [edit]})` |
| Replace a [plain paragraph scalar range](docs/EDITOR-PARITY.md#character-ranges-in-plain-body-paragraphs) | `preview_with_ranges` / `TextRangeEdit` | `preview([], text_ranges=[edit])` | `preview({textRanges: [edit]})` |
| Replace text in a direct body run | `doc.preview_with_text(&[], &[], &[], &[edit])?` | `doc.preview([], run_text=[edit])` | `doc.preview({runText: [edit]})` |
| Remove a direct body run | `doc.preview_with_run_removals(&[], &[], &[], &[], &[anchor])?` | `doc.preview([], run_removals=[anchor])` | `doc.preview({runRemovals: [anchor]})` |
| Inspect removed runs | `preview.run_removals()` | `preview.run_removals` | `preview.runRemovals()` |
| Inspect changed runs | `preview.run_text_changes()` | `preview.run_text_changes` | `preview.runTextChanges()` |

Entries include the original ID, note type or comment
metadata, and direct paragraphs with snapshot-scoped inline anchors.
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
- **Compose** with an unsigned Transitional DOCX or DOTX template; output is DOCX. **Inspect and preview edits** on an unsigned DOCX using targets from the same snapshot. Image appends support effective default, first and even headers **and footers**. A shared edit affects every alias of the physical part; a section-local edit forks the selected slot. See the [editor inventory](docs/EDITOR-PARITY.md) for target and preservation limits.
- [Template replacement](https://reindervosdewael.github.io/rocx/recipes/#edit-an-existing-document) can span runs within a paragraph or hyperlink. It cannot cross structured boundaries or change fields, revisions or controls. Replacement text belongs to the first matched run; bookmark/comment markers and untouched break attributes stay in place.
- Untouched **part payloads** retain their bytes. Edited XML and the ZIP container may be reserialized. Related parts are found by relationship, not filename; invalid internal relationships fail. Path output replaces the destination only after compilation, using a sibling temporary file.
- Packages declaring a digital-signature origin are rejected, even without edits. Use an unsigned copy; rocx does not strip, verify or renew signatures.
- On edited parts, unsupported or unbound `mc:MustUnderstand` namespaces on WordprocessingML elements are rejected. The guard recognizes Transitional WordprocessingML, Office relationships and XML. It does not select `AlternateContent` branches or scan foreign subtrees and untouched opaque parts.
