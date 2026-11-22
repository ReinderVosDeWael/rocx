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

This builds/runs both examples, compares complete DOCX ZIP bytes, checks source
template bytes too, verifies independently loaded content and prints a SHA-256.
Output:

- `examples/all_features-rust.docx`
- `examples/all_features-python.docx`
- `examples/all_features-rust.template.docx` and
  `examples/all_features-python.template.docx` (small intermediate templates).

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

Verified local baseline through R8n: both final files are 58,774 bytes, SHA-256
`18e99b3262acd2bad24d83b92497127d6abbf2bf52d310f78ea3d8f4b5b38add`.

The gallery also labels a standalone 144-by-48-point inline text box. It uses a
direct DrawingML text shape, plain multiline text, Arial 12 pt and explicit
bold=false, with no geometric fill/outline. This does not demonstrate a rich
WordprocessingML text-box story or floating placement.

The inline-shape gallery labels the rectangle's 14.25-point white Arial text,
explicit bold=false, italic=true, horizontal centering and bottom anchoring. It preserves a blank
middle line. The adjacent ellipse has no text body; neither sample claims text
fitting or manually verified placement in Office.

Topic 02 labels a chainable-setter sample: bold omitted, bold enabled, and bold
explicitly disabled with italic enabled. Rust uses `RunFormat`/`ParagraphFormat`
builders; Python uses equivalent existing keyword arguments. The inventory guard
also checks that public Rust authoring value fields have adjacent setters and
assignment tests. Builders change authoring syntax only; they do not add OOXML
features. See [the builder contract](../docs/FORMATTING.md#rust-chainable-authoring-setters).

Topic 04 also labels all five inline-table alignment tokens on narrow 2160-twip
tables. Existing tables retain omitted alignment; paragraph alignment is separate.

Topic 06 additionally shows legal-numbering display requests on an abstract level
and concrete replacements, including explicit false. The viewer computes labels;
the catalog checks preserve `numFmt` and inspect `isLgl` independently.
It also labels tab/space/nothing suffix requests on abstract/replacement levels
and explains omitted defaults. Separators are consumer-generated, not text edits.
It labels level-restart requests separately from start overrides: zero means
never restart; trigger 1 is one-based; omitted levels retain consumer defaults.
Concrete instances 61 and 62 share abstract bullet definition 100. Instance 62
adds its own start override. The checks verify both references and a single
Abstract and replacement levels also show distinct bold and italic label requests.
The same levels show distinct left and hanging indent requests in points.
They also associate different registered heading styles with the two levels.
The Catalog Bullets numbering style points to concrete instance 61, with a reciprocal link on abstract definition 100.
Concrete instance 63 inherits from Catalog Bullets through a level-free abstract definition.
Topic 06 also labels separate document-wide placement, formats, starts and restart modes for footnotes and endnotes.
It also labels tracked text insertions and deletions with review metadata and an explicitly disabled future-edit tracking flag.
The template stage accepts imported text changes and a paired move before adding new revisions.
The authored move sample labels the common source and destination name.
The inline rich-text control sample shows its alias, tag, lock and formatted content.
A separate block control groups one paragraph and one table. Both controls label their explicit IDs.
A third control labels plain text with multiline enabled.
The drop-down sample labels its choices and selected value.
The table sample labels separate wrappers for a row and one of its cells.
The generated template also includes a plain-text control whose cached value
is updated by tag while its bold formatting is preserved.
abstract definition.

It also labels five numbering-label alignments on abstract levels, start/end on
replacement levels, and omission elsewhere. Label alignment does not set paragraph
text alignment; the viewer performs label positioning.
Both pass official schema validation for 17 covered parts plus OPC relationships.
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
| Defaults and Properties dialog | Run defaults (Calibri 11 pt), paragraph defaults (4 pt after, 1.1 lines); creator/title/subject/description/keywords/category/comments/version; fixed snapshot/render timestamps. Defaults remain separate from direct content properties. |
| 01 Paragraphs/headings | Heading 0/no heading through 9; all nine alignments; multiplier/exact/minimum line heights and legacy line spacing; before/after spacing; left/right/first-line/hanging point indents; keep-lines/next, widow control, page-break-before and explicit false values. |
| 01 Tabs | All eight alignments including start/end/bar/clear; all six leaders; signed positions and explicit clear at −12 pt. |
| 02 Runs | Bold/italic/strike/caps/small-caps/super/subscript; false overrides; literal fonts and point sizes; RGB/automatic color; all 16 theme slots with tint/shade; all seven underline styles; Unicode, preserved spaces, empty runs, literal tabs/newlines. |
| 02 Script requests | Independent paragraph bidi, run RTL/complex-script flags, complex-script bold/italic/size, three language slots and literal East Asian/complex-script fonts; true/false cases. |
| 03 Styles | Forward paragraph/character bases, next style and reciprocal opposite-kind links; direct character reference/override; legacy paragraph-style inch indents; update existing template paragraph/character styles while retaining unspecified bold. No computed effective formatting. |
| 04 Tables | Fixed widths, explicit autofit and default allocated grid; all six table-border sides, all four cell-border sides, both line kinds; shading; top/center/bottom cell alignment; all margin sides including zero; horizontal spans and vertical restart/continue; short-row padding; nested tables with trailing paragraphs; all-disabled TableLook (mask 0600). |
| 04 Rows/text flow | Exact/minimum heights, repeat-header/cannot-split true/false; 36-row pagination sample; all six cell text directions. |
| 05 Conditional styles | Separate labeled tables for all 13 region inputs, including base whole-table; all six TableLook activation flags explicitly enabled (mask 01E0); run/font/color/script/language requests, paragraph alignment/spacing/line-height/tabs/bidi and cell shading. Properties stay in style definitions, not direct cells. |
| 06 Lists | Nine levels/references, decimal/lowerRoman/bullet formats, abstract starts and literal text, concrete level-0 restart at 7 and level-8 override at zero; concrete level-0 lowerLetter replacement combined with restart, and level-1 upperRoman replacement-only. Abstracts remain unchanged. Format/text are open strings, not finite enum galleries. |
| 06 Links/ranges | External URL with tooltip/history and explicit display formatting; internal bookmark link; overlapping/cross-paragraph bookmarks and comment range; history=false; text/image/tab/break hyperlink children. Header/comment/note links use owner-local relationships. |
| 06 Comments | Plain cross-paragraph range; structured paragraph/table/image/link story with author/date/initials; paragraph/run convenience comments with author overrides. Inspect the comments pane. |
| 06 Notes | Footnote/endnote references and distinct kind/id=0 definitions; rich footnote paragraph/table/image/link story and endnote body. Automatic special note entries are infrastructure, not ordinary visible notes. |
| 06 Fields | PAGE/NUMPAGES/REF/TOC/raw DATE in simple and complex forms; nested cache, dirty/locked true/false. Caches are labeled placeholders, not evaluated values. |
| 07 Images | PNG/JPEG/GIF/BMP/TIFF; density-derived/native/zero-DPI sizes; width-only, height-only and two-dimensional transforms; accessibility title/description/name; byte deduplication; filesystem path and async bytes/deferred transformation/alt text. `type` is an ignored hint, not an image converter. |
| 07 Breaks | Line/textWrapping/page/column variants with before/after labels. Literal tab/newline and explicit Tab/Break are shown separately. |
| 08 Authoring | Ready/lazy/async values; named property references and explicit Deferred; true/false conditions with an excluded producer that would fail if run; immutable resolved block/inline reuse; repeated snapshot renders must match. |
| 09 Sections | All five break kinds; portrait/landscape; page dimensions/margins; two columns/spacing/separator; page-number start/format; all four vertical alignments; title-page true/false; section bidi/gutter-side true/false independent of text. |
| Header/footer stories | Default/first/even headers and footers, title-page selection, header image/link and PAGE/NUMPAGES footer caches, all visibly labeled. |
| Generation/check scripts (nonvisual) | Explicit Transitional profile/read budgets; template loading; bytes/path save; fixed metadata; OPC relationships/validation; asset deduplication; untouched opaque theme-byte retention. Rust additionally demonstrates caller-owned-byte compilation and `PackagePlan.parts/part/validate/to_bytes`. Python's optional `to_docx` convenience uses the same bytes; verification independently opens them with python-docx without resaving. |

Alternative aliases/builders, failed-input diagnostics, cancellation, cycle
rejection, hostile ZIP budgets, relocated namespaces and arbitrary opaque
extensions have dedicated tests; invalid structures do not belong in a visual
catalog. Lower-level Rust identity/measurement helpers support authoring and
packaging but have no separate visual effects. This covers the currently
implemented surface, not future roadmap features or all possible raw field,
number-format and language strings.

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

Section samples in topic 09 label all three line number restart modes and
show preservation and explicit removal of previous settings. The samples also
label paragraph suppression enabled and explicitly disabled.
The first section also labels independently authored header and footer edge distances.
The document settings sample labels mirrored margins and an explicitly disabled top gutter.
It shows single top/bottom and dashed left/right page borders, offset from
the page edges and placed behind the text on the first page.
The later sections label all four document grid modes, with line pitch in
twips and a signed character-pitch delta. They also label paragraph snapping
enabled and run snapping explicitly disabled.
The numbered sections also label contextual spacing on and mirrored paragraph
indents explicitly off.

Topic 07 labels a cropped, rotated and flipped inline picture and a floating picture
with explicit page/margin offsets, square wrapping with distances, and placement flags.
A second floating picture shows tight wrapping with an explicit contour.
The inline shape sample labels a solid blue rectangle with text and a solid green ellipse with an outline.
