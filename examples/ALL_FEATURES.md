# All-features visual inspection catalog

These are independent authoring programs using the public APIs, not wrappers
around a shared prebuilt document or XML fixture:

- [Rust entrypoint](all_features.rs), with [support modules](showcase/).
- [Python program](all_features.py).

## Generate and verify

From the repository root, using the provisioned development environment:

```sh
UV_PROJECT_ENVIRONMENT=.venv-opencode TMPDIR=/tmp/opencode uv run --no-sync python tools/check_examples.py
```

This builds/runs both examples, compares complete DOCX ZIP bytes for the final,
template and batch-edited catalogs and the three header-scope artifacts, verifies
content and prints SHA-256 hashes for the main catalogs and header-scope artifacts.
Output:

- `examples/all_features-rust.docx`
- `examples/all_features-python.docx`
- `examples/all_features-rust.template.docx` and
  `examples/all_features-python.template.docx` (small intermediate templates).
- `examples/all_features-rust.edited.docx` and
  `examples/all_features-python.edited.docx` (separate imported edits).
- Paired `.header-source.docx`, `.header-shared.docx`, and `.header-local.docx`
  artifacts: a simple two-section source and two independent transactions,
  each combining an alignment patch, a table insertion, and a header image.

Generated documents are ignored by Git. Copy them elsewhere to retain a visual
baseline before regenerating. Append `--output-dir /path/to/directory` to choose
another directory. Run either authoring program alone:

```sh
cargo run --locked --example all_features -- examples/all_features-rust.docx
UV_PROJECT_ENVIRONMENT=.venv-opencode uv run --no-sync python examples/all_features.py examples/all_features-python.docx
```

After native changes, rebuild the editable Python extension first using the
repository's locked native rebuild command. A stale native extension can cause
an actual parity failure.

Both generated documents pass official schema validation for 20 covered parts
plus OPC relationships.
Future intentional feature additions will change this baseline; equality is
checked against the other implementation, not against a permanently fixed hash.

## Inspection guide and coverage inventory

Each sample names its requested feature in ordinary text before/within it.
Heading samples say **“This is a Heading 1”** through **“This is a Heading 9”**;
numbered topic headings are separate navigation aids. The machine-readable
[declaration/field inventory](showcase/coverage.json) guards API drift. It lists
supported fields, not every Cartesian combination in every story/input form.

| Location | Functionality exercised |
| --- | --- |
| Start and retained template paragraph | Two-stage template creation, insertion at paragraph index 1, cross-run replacement `{{replace}} → REPLACED`, supported-story edit scope and retained template content. |
| Template safety | The catalog edits an unsigned template. Both APIs reject templates declaring an OPC signature origin; rejection is tested separately because it does not produce a document. The template stage also accepts imported text changes and a paired move before adding new revisions. |
| Custom document properties | `Project` is retained as `Atlas`; `Review status` changes from `Pending` to `Draft & review`; the boolean `Review complete` changes from false to true, and `Published` is added as an explicit false. |
| Template compatibility | The catalog labels rejection of unsupported mandatory namespaces on edited part roots and ordinary WordprocessingML descendants. Rejection is tested separately because it produces no document. |
| Extended document properties | Inspect the document's metadata: Company is `Atlas & Co.` and Manager is `Taylor`. |
| Defaults and Properties dialog | Run defaults (Calibri 11 pt), paragraph defaults (4 pt after, 1.1 lines); creator/title/subject/description/keywords/category/comments/version; fixed snapshot/render timestamps. Defaults remain separate from direct content properties. |
| 01 Paragraphs/headings | Heading 0/no heading through 9; all nine alignments; multiplier/exact/minimum line heights and legacy line spacing; before/after spacing; left/right/first-line/hanging point indents; keep-lines/next, widow control, page-break-before and explicit false values. |
| 01 Imported editor | Both APIs inspect the generated body, find the labeled left-aligned paragraph by its text, confirm its typed inline text, and preview an anchored center-alignment patch and a declarative two-column fixed-width table immediately after it in one batch. Both reopen the edited package and inspect the physical table row, cell, and inline text. The inserted table has a bold label and appears only in the separate edited catalog. Other package parts retain their exact payload bytes. |
| Combined body and default-header transaction | One default header is shared by two sections. Both APIs resolve the labeled header run and hyperlink's snapshot-scoped read-only anchors with `inline_at`. Inherited aliases of the same physical part yield equal inline anchors; the image edit target is still section-scoped. Both previews center the labeled body paragraph and insert a table immediately after it, in the same transaction as the header edit targeting the second section. Shared mode changes the header used by both; section-local copies it and changes only the second section. Exact byte parity and package checks cover all three artifacts, including body changes, part-local relationships, image counts, and untouched payloads. |
| 01 Tabs | All eight alignments including start/end/bar/clear; all six leaders; signed positions and explicit clear at −12 pt. |
| 02 Runs | Bold/italic/strike/caps/small-caps/super/subscript; false overrides; literal fonts and point sizes; RGB/automatic color; all 16 theme slots with tint/shade; all seven underline styles; Unicode, preserved spaces, empty runs, literal tabs/newlines. A labeled chainable-setter sample shows omitted, enabled and explicitly disabled bold (with italic enabled); builders alter Rust authoring syntax only, not OOXML features. See [the builder contract](../docs/FORMATTING.md#rust-chainable-authoring-setters). |
| 02 Script requests | Independent paragraph bidi, run RTL/complex-script flags, complex-script bold/italic/size, three language slots and literal East Asian/complex-script fonts; true/false cases. |
| 03 Styles | Forward paragraph/character bases, next style and reciprocal opposite-kind links; direct character reference/override; legacy paragraph-style inch indents; update existing template paragraph/character styles while retaining unspecified bold. No computed effective formatting. |
| 04 Tables | Fixed widths, explicit autofit and default allocated grid; all six table-border sides, all four cell-border sides, both line kinds; shading; top/center/bottom cell alignment; all margin sides including zero; horizontal spans and vertical restart/continue; short-row padding; nested tables with trailing paragraphs; all-disabled TableLook (mask 0600). Narrow 2160-twip tables label all five inline-table alignment tokens; existing tables retain omitted alignment, and paragraph alignment is separate. |
| 04 Rows/text flow | Exact/minimum heights, repeat-header/cannot-split true/false; 36-row pagination sample; all six cell text directions. |
| 05 Conditional styles | Separate labeled tables for all 13 region inputs, including base whole-table; all six TableLook activation flags explicitly enabled (mask 01E0); run/font/color/script/language requests, paragraph alignment/spacing/line-height/tabs/bidi and cell shading. Properties stay in style definitions, not direct cells. |
| 06 Lists | Nine levels/references, decimal/lowerRoman/bullet formats, abstract starts and literal text, concrete level-0 restart at 7 and level-8 override at zero; concrete level-0 lowerLetter replacement combined with restart, and level-1 upperRoman replacement-only. Abstracts remain unchanged. Format/text are open strings, not finite enum galleries. Legal-numbering requests (including false), suffix choices, level restarts (zero never restarts; trigger 1 is one-based), start overrides, label alignment, bold/italic, point indents and registered heading styles are labeled on abstract/replacement levels; omitted levels retain consumer defaults. Concrete instances 61/62 share abstract bullet definition 100, with a start override on 62; Catalog Bullets links instance 61 reciprocally to abstract 100, and instance 63 inherits through a level-free abstract definition. Separators and label positioning are consumer-generated; the viewer computes labels, while checks inspect `numFmt` and `isLgl`. |
| 06 Links/ranges | External URL with tooltip/history and explicit display formatting; internal bookmark link; overlapping/cross-paragraph bookmarks and comment range; history=false; text/image/tab/break hyperlink children. Header/comment/note links use owner-local relationships. |
| 06 Imported editor inline structure | The labeled paragraph has one direct run followed by a bookmark hyperlink containing two separate runs. Both editors inspect the boundaries and read nodes back by their snapshot-scoped anchors. Coverage checks the original and batch-edited packages and rejects merged runs even when the visible text is unchanged. The addresses do not support inline edits. |
| 06 Comments | Plain cross-paragraph range; structured paragraph/table/image/link story with author/date/initials; paragraph/run convenience comments with author overrides. Inspect the comments pane. |
| 06 Notes | Footnote/endnote references and distinct kind/id=0 definitions; rich footnote paragraph/table/image/link story and endnote body. Document-wide placement, formats, starts and restart modes are labeled separately. Automatic special note entries are infrastructure, not ordinary visible notes. |
| 06 Fields | PAGE/NUMPAGES/REF/TOC/raw DATE in simple and complex forms; nested cache, dirty/locked true/false. Caches are labeled placeholders, not evaluated values. |
| 06 Revisions and controls | Tracked insertions/deletions with review metadata and future-edit tracking explicitly disabled; authored move with common source/destination name. Labeled inline rich-text control (alias, tag, lock, formatted content), block control (paragraph and table), multiline plain-text control, drop-down choices/selection, and table row/cell wrappers; explicit IDs are shown. Generated template also updates a cached plain-text control by tag while preserving bold. |
| 07 Images | PNG/JPEG/GIF/BMP/TIFF; density-derived/native/zero-DPI sizes; width-only, height-only and two-dimensional transforms; accessibility title/description/name; byte deduplication; filesystem path and async bytes/deferred transformation/alt text. `type` is an ignored hint, not an image converter. Topic 07 also labels a cropped, rotated, flipped inline picture and floating pictures with page/margin offsets, square wrapping/distances/placement flags or tight wrapping with explicit contour. |
| 07 Breaks | Line/textWrapping/page/column variants with before/after labels. Literal tab/newline and explicit Tab/Break are shown separately. |
| 07 Charts | One workbook-free inline column series (North 8, South 13.5, West 5). |
| 07 Equations | A stacked 1-over-2 Office Math fraction between ordinary text runs. |
| 07 Shapes | A standalone 144-by-48-point inline DrawingML text box with plain multiline text, Arial 12 pt and explicit bold=false, without geometric fill/outline; this is not a rich WordprocessingML text-box story. A separate floating text box places plain text 220 pt from the margin and 12 pt below its paragraph, with square wrapping and a 6 pt left gap. The inline sample labels a solid blue rectangle with 14.25-point white Arial text, explicit bold=false, italic=true, horizontal centering, bottom anchoring and a blank middle line; the solid green ellipse has an outline but no text body. The shape samples do not claim text fitting or manually verified Office placement. |
| JavaScript authoring | The labeled centered paragraph, rectangular table mixing string cells and block-based cells (formatted runs, a tab, external link, line break, and nested table), and sized inline PNG with alt text mirror the [Node/browser example](../javascript/README.md). The standalone report is compared byte for byte across Rust, Python, Node, and browser; the JavaScript binding does not author the full catalog. |
| 08 Authoring | Ready/lazy/async values; named property references and explicit Deferred; true/false conditions with an excluded producer that would fail if run; immutable resolved block/inline reuse; repeated snapshot renders must match. |
| 09 Sections | All five break kinds; portrait/landscape; page dimensions/margins; two columns/spacing/separator; page-number start/format; all four vertical alignments; title-page true/false; section bidi/gutter-side true/false independent of text. Samples label all three line-number restart modes, preservation/removal of prior settings, paragraph suppression enabled/disabled, independent header/footer edge distances, mirrored margins, disabled top gutter, page-border edges/offset/display and all four grid modes with line/character pitch. Paragraph snapping is enabled, run snapping disabled; contextual spacing is on and mirrored paragraph indents are off. |
| Header/footer stories | Default/first/even headers and footers, title-page selection, header image/link and PAGE/NUMPAGES footer caches, all visibly labeled. |
| Generation/check scripts (nonvisual) | Explicit Transitional profile/read budgets; template loading; bytes/path save; fixed metadata; OPC relationships/validation; asset deduplication; untouched opaque theme-byte retention. Rust additionally demonstrates caller-owned-byte compilation and `PackagePlan.parts/part/validate/to_bytes`. Python's optional `to_docx` convenience uses the same bytes; verification independently opens them with python-docx without resaving. |

Use `ImportedDocument::preview_transaction` in Rust or `ImportedDocument.preview`
with `table_insertions=` and `header_images=` in Python. The preview reports body
and header changes separately and produces one DOCX. Targets must come from the
original snapshot. Duplicate operations of the same kind at a target and shared
edits that collide on a physical source header are rejected. Distinct local edits
may fork the same original header independently; an adjacent local edit takes
precedence over restoring inheritance at its section. Failed batches return no
preview and leave the source unchanged.

Alternative aliases/builders, failed-input diagnostics, cancellation, cycle
rejection, hostile ZIP budgets, relocated namespaces and arbitrary opaque
extensions have dedicated tests; invalid structures do not belong in a visual
catalog. Lower-level Rust identity/measurement helpers support authoring and
packaging but have no separate visual effects. This covers the currently
documented authoring surface, not every possible raw field, number-format or
language string.

## Viewer caveats

- Open either final file: the initial packages are byte-identical. Saving or
  updating fields in a viewer normally changes them; keep untouched originals.
- This intentionally spans many pages. Even/odd section breaks may insert blank
  pages; exact heights can clip text; vertical text/shaping/font fallback depends
  on installed fonts and the viewer.
- Fields, page numbers, list labels and TOC layout are not calculated by this
  library. Update fields if desired, but preserve an original.
- Table defaults still enable first-row/first-column and horizontal banding, not
  last-row/last-column or vertical banding. Topic 05 now uses explicit TableLook
  controls to enable all six requests. The viewer still decides conditional
  precedence and visible layout; the library does not compute effective styles.
- `rtl_gutter` requests placement, not a gutter amount. Section bidi, paragraph
  bidi, run RTL and cell text flow remain independent.
- Light theme colors may be invisible on white; their labels stay uncolored.
- Schema/byte checks are not a manual rendering result.

## Maintenance contract

Every new feature must update both programs, this inventory, the machine
inventory when APIs change, and independent `tools/check_examples.py` coverage
assertions before being marked complete. Keep descriptions visible/literal and
timestamps/assets/declaration order aligned.

`test_all_features.py` runs both programs and checks exact ZIP equality, expected
loaded content, container-only mismatch and removed-feature negative controls.
`test_standards.py` validates the complete catalog with official schemas and
bad border/conditional-order mutations. `tests/showcase.rs` runs the Rust catalog
without Python and checks combined property order. Nothing is normalized,
unpacked/repacked or rewritten before byte comparison; ZIP diagnostics are only
used after a failure.
