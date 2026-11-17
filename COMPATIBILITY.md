# Declarative compatibility

Target: Child Mind Institute `cmi-docx` 0.6.16, revision
`3449b942034129ad74a28aeb253361035806e385`. The goal is the declarative interface;
Python objects, Python's imperative wrappers, and byte-identical ZIP archives
are outside this target.

## Inventory

| Component | Implemented surface |
| --- | --- |
| `Component` | Condition, lazy fields, asynchronous fields/children, concurrent resolution, standalone resolution/await |
| `Document` | Sections; creator/title/subject/description/keywords/category/comments/version; comment author; style definitions; numbering; DOCX bytes/save; templates |
| `DocumentTemplate` | Path, ordered replacements, original paragraph insertion index |
| `Section` | Conditional/deferred block list, properties, default/first/even headers and footers |
| `SectionProperties` | Page size/orientation/margins, page numbering, columns, vertical alignment, title page, break type |
| `Header`, `Footer` | Conditional/deferred block lists, including tables and inline images |
| `Paragraph` | Exclusive text/children, heading/style/alignment, before/after/line spacing, left/right/first-line indents, keep together/next, page break before, widow control, comments |
| `TextRun` | Text, comments and author; bold/italic/underline/font/size/color, superscript/subscript/strike/all caps/small caps |
| `Tab`, `Break` | Conditional inline tab; line/text-wrapping/page/column break |
| `ImageRun` | Bytes/path/async data, type field, point-based sizing, accessibility metadata; PNG/JPEG/GIF/BMP/TIFF |
| `Table` | Conditional/deferred rows, column widths, autofit/fixed layout, named style, borders |
| `TableRow` | Conditional/deferred cells |
| `TableCell` | Conditional/deferred blocks and nested tables, grid span, vertical merge, borders, background, vertical alignment |
| `TableBorder`, `CellBorder` | Side, size, RGB color, line type, uppercase hexadecimal color |
| `ParagraphStyleDefinition` | Name, base/next style, font properties, paragraph properties, create-or-update semantics |
| `TableStyleDefinition` | Name/base style, whole table, first/last row/column, both row/column bands, four corners; duplicate names rejected |
| `TableSectionFormat` | Font/size/bold/italic/underline/color/background/alignment/before/after spacing |

Mixed children are `Block` and `Inline` enums. Header/footer types, alignments,
breaks, merges, and border sides use Rust enums rather than Python enums/string
literals. Records such as `PageSize`, `PageMargins`, `Columns`,
`ImageTransformation`, and `AltText` replace Python dictionaries. RGB tuples are
`[u8; 3]`. Style definitions use `.into()` to construct a `StyleDefinition`.

## Units and defaults

| Field | Unit |
| --- | --- |
| Run font size, paragraph spacing/indents, image transformation | Points |
| Paragraph style left/right/first-line indent | Inches |
| Section page size and margins | EMU: 914400 per inch |
| Table column widths, column spacing | Twips: 1440 per inch |
| Border size | Eighths of a point |
| Line spacing | Multiplier |

The bundled blank template supplies the same Normal, Heading, table, list,
numbering, theme, font, and default page settings as python-docx 1.2.0. Default
page geometry is 8.5 × 11 inches with 1.25-inch left/right margins. Section
properties inherit unspecified values. All predicate defaults are true.
Each CR and each LF produces a break, so CRLF becomes two line breaks. Explicit
empty text runs survive; empty shorthand paragraphs have no runs.

## Resolution and validation contract

Rust resolution consumes declarations once and creates immutable snapshots;
resolved documents can render repeatedly. Conditions are synchronous `FnOnce`
closures or booleans; deferred producers/futures require owned `Send + 'static`
captures. The first observed error drops remaining futures. With
`use cmi_docx::Component`, individual components return an optional resolved
snapshot; `Document` returns a complete resolved document. Awaiting directly uses
the same interface. Metadata and formatting fields have the declared static CMI
types; deferred text, comments, child lists, and components use `Value<T>`.
Python's incidental ability to assign arbitrary dynamic types to any dataclass
field is not a Rust API guarantee.

Invalid paragraph content, illegal XML characters, invalid image headers,
width/grid mismatches, unknown styles, duplicate table styles, non-finite
formatting, and invalid package parts return errors. Text errors identify the
field and UTF-8 offset. Rendering performs validations requiring template/style
context, before any output file is opened.

## Deliberate differences and upstream omissions

- Predicates run once, rather than being repeatedly called during Python packing.
  Excluded final sections do not leave a trailing blank section.
- The first paragraph in a cell obeys its condition. The pinned Python packer
  can bypass that condition while filling the initial empty cell paragraph.
- All child dictionaries, deferred components, headers/footers, and comments are
  recursively resolved. The pinned resolver incompletely traverses dictionaries
  and components returned by coroutines.
- Page numbering, columns, vertical alignment, title-page/break-type options,
  image accessibility metadata, and whole-table background are rendered. The
  pinned implementation declares these fields but ignores some of them.
  Supplying even-page headers enables even/odd headers in settings.
- Document `numbering` is functional: `NumberingDefinition` contains levels and
  an ID, and paragraphs can reference it through `NumberingReference`. The
  pinned package stores its numbering dictionary without using it.
- Like the pinned image backend, the library accepts PNG, JPEG, GIF, BMP, and
  TIFF; SVG is rejected despite the upstream docstring listing it. The upstream
  `ImageRun.type` field is retained but content is detected from image bytes.
- Template replacements are an ordered vector of pairs, preserving Python
  dictionary insertion order. Empty search needles are rejected.
- Boundaries between sections and template paragraph insertion follow the Python
  packing order. Header/footer changes append to linked existing stories.
- OOXML representations can differ in equivalent defaults, namespace prefixes,
  property order, relationship IDs, and comment marker order at the same run
  boundary. Document timestamps use UTC at resolution, rather than each render.

## Evidence

`cargo test --features python-tests --test python_docx` generates DOCX files in
Rust and invokes `tests/python_docx/verify.py` through `uv run --locked --group test`
to read them with python-docx 1.2.0. Python and dependencies are managed by
`.python-version`, `pyproject.toml`, and `uv.lock`.
Independent expected values check public reader properties for text, runs,
formatting, styles, tables, images, comments, metadata, and sections; properties
without a public accessor use python-docx's XML model. Template tests read both
the edited file and its original to check preservation. These tests need no CMI
checkout. Four mutation tests make 24 targeted edits to generated files and
require Python to open each file, then fail on the specific expected-value
assertion. This covers text/run preservation, formatting, table properties,
image content attached to the correct shape, comment anchor spans, metadata,
layout, and template content. A parse/import/setup failure cannot satisfy the
rejection check.

`tests/fixtures/generate_reference.py` produces the original plain-text/run
fixtures. `generate_features.py` builds `features.docx` from the pinned checkout
and can compare a Rust-generated document through python-docx readback. It
compares paragraph/run formatting, custom styles, nested and merged tables,
section geometry, headers/footers, image sizes, comment text/authors, and core
metadata. Rust tests also compare normalized OOXML, custom style definitions,
and comment anchor ranges with that reference.

Separate tests cover every table style region, all supported image headers and
DPI handling, fixed/autofit widths, the additional declared section properties,
async field concurrency and exclusions, numbering, cross-run template
replacement, nested stories, insertion positions, existing comments/media,
unrelated ZIP parts, minimal templates, UTF-8/UTF-16 XML, invalid inputs, and failure-safe output.
These tests establish parity for the declared feature surface and the recorded
Rust contracts; visual pagination across different Word applications is not
asserted byte-for-byte.
