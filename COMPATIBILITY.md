# Current support and conventions

Version 0.3 follows its own declarative domain model and documented behavior.
Constructor signatures and consumer-specific quirks are not compatibility
requirements.
See [migration](docs/MIGRATION.md), [module boundaries](docs/MODULES.md), and the
[OOXML specification/coverage map](docs/OOXML.md).

## Implemented surface

| Area | Current support |
| --- | --- |
| Declarations | Documents, sections, paragraphs, runs, tables, headers/footers; ready/lazy/async fields and synchronous conditions; deferred Rust metadata/format/property aggregates and immutable resolved-child reuse |
| Dependencies | Rust typed named properties and shared computations; Python per-field references and shared computations; typed graph cycles and cancellation handling |
| Text | Unicode, whitespace, explicit empty runs, tabs, line/page/column breaks |
| Hyperlinks | Rich external absolute URI/IRI links in body/table/header/footer paragraphs, with deferred targets/children/tooltip/history; text/image/tab/break children; owner-local external relationships, never fetched |
| Bookmarks/ranges | Typed paired bookmark/body-comment markers, overlap and cross-paragraph ranges in one story, internal bookmark links, imported name/ID reservation; see [range contract](docs/RANGES.md) |
| Notes | Footnote/endnote definitions and body references, rich block stories, type-distinguished special entries, imported ID/owner checks; no layout/number calculation; see [notes](docs/NOTES.md) |
| Formatting | Selected paragraph/run properties, RGB/theme/automatic run colors, underline styles, explicit false overrides; typed line heights and signed custom tab stops/leaders/clears on paragraphs/styles ([coverage](docs/FORMATTING.md)) |
| Styles | Document run/paragraph defaults, paragraph/character/table definitions and selected conditional table regions (including line heights/tabs/bidi and run script controls), direct character run references, forward base/next/opposite-kind linked references, reachable inheritance-cycle/kind checks and per-compilation catalog ([contract](docs/STYLES.md)) |
| Tables | Nested tables, fixed/autofit widths, spans/vertical merges, borders, shading and vertical alignment; exact/minimum row heights, repeated-header/cannot-split, inline alignment and explicit conditional table-look activation requests ([contract](docs/TABLES.md)) |
| Sections | Geometry, margins, page borders, header and footer distances, columns, page and line numbering, document grids, break type, title-page, vertical alignment, optional section page direction and right-side gutter request ([contract](docs/FORMATTING.md)) |
| Stories | Main body, default/first/even headers and footers; comment generation currently restricted to body context |
| Comments | Text conveniences or structured paragraph/table/image/link content on paired body ranges; author/date/initials metadata; no threaded comments |
| Images | PNG/JPEG/GIF/BMP/TIFF metadata and inline or anchored pictures, density-aware dimensions and accessibility fields; deferred data/transformation/alt text in both APIs; header inspection without pixel decoding |
| Shapes | Inline or floating DrawingML rectangles, ellipses and plain text boxes in body paragraphs, with checked dimensions and optional text styling; geometric shapes also support RGB fill and outline ([contract](docs/FORMATTING.md#floating-shapes-and-text-boxes)) |
| Charts | One workbook-free inline column series in body paragraphs, including body table cells ([contract](docs/FORMATTING.md#inline-column-charts)) |
| Equations | One stacked inline fraction with literal text operands in body paragraphs, including body table cells ([contract](docs/FORMATTING.md#inline-fractions)) |
| Numbering | Native definitions/levels/references, shared authored abstract IDs and authored numbering styles, concrete per-level start overrides and level replacements, optional legal-numbering, suffix, level-restart, label-alignment, label-formatting and level paragraph formatting and style links in Rust and Python; checked level references, no counter evaluation ([coverage](docs/NUMBERING.md)) |
| Fields | Simple/balanced complex paragraph-local fields, nested cached results, typed PAGE/NUMPAGES/REF/TOC and raw instructions; no evaluation or multi-paragraph TOC generation; see [fields](docs/FIELDS.md) |
| Revisions | Authored text-run insertions, deletions and paired moves with metadata; optional future-edit tracking; scoped accept/reject for selected imported text changes ([contract](docs/REVISIONS.md)) |
| Content controls | Inline, block, row and cell text controls with alias, tag, lock, optional ID and rich-text, plain-text and drop-down modes. Selected imported plain-text controls can be edited by tag ([contract](docs/CONTENT_CONTROLS.md)) |
| Metadata | Core properties, snapshot timestamp, company and manager, and named string and boolean custom properties ([contract](docs/OOXML.md#document-properties)) |
| Templates | Relationship-driven part discovery, namespace-safe rewriting, opaque payload retention, structural text replacement and insertion |
| Packaging | Typed names/targets/IDs, defaults/overrides, read budgets, internal relationship/reference graph checks and inspectable package plan |

Other revision contexts and content control types, additional drawing and chart
features, diagrams, broader equation support, and Word extensions are outside
the current support profile. Retaining imported data is not a claim of
typed support. Strict output is explicitly unsupported; the writer uses the
Transitional vocabulary. No Word layout or field-calculation engine is included.

Hyperlinks do not imply blue/underlined styling; format their display runs
explicitly. Nested hyperlinks are a deliberate authoring limitation, although
the Transitional `CT_Hyperlink` schema permits them. Targets retain percent
escapes, Unicode, queries and fragments verbatim. They must have an absolute
scheme, no whitespace/control/forbidden URI characters, and valid percent escapes;
this lexical check is not a full URL parser, scheme allowlist or safety check for
opening links. Callers must apply their own policy to untrusted targets. Relative
URI targets are unsupported; internal links require an explicit typed bookmark
target and do not create external relationships.

## Units and defaults

| Current authoring convenience | Unit |
| --- | --- |
| Run size, paragraph spacing/indents, image transformations | Points |
| Paragraph style left/right/first-line indent | Inches |
| Section page sizes/margins | EMU: 914400 per inch |
| Table widths and column spacing | Twips: 1440 per inch |
| Border size | Eighths of a point |
| Line spacing | Multiplier |

Checked `Points`, `Twips`, `Emu` and `HalfPoints` values provide physical
conversions; codecs validate numeric convenience values before encoding. Domain
units and application read limits are distinct from each element's schema bounds.
The blank template supplies default styles/themes/page settings, including an
8.5 × 11 inch page with 1.25 inch left/right margins. Its provenance and one
schema-required zoom correction are recorded in [assets](assets/README.md).

## Resolution, editing and output

Rust consumes declaration programs. Python leaves declarations unchanged and
returns immutable snapshots. Factories support fresh passes; bare futures/
coroutines are single-use. Rendering snapshots never re-executes producers.
Conditions are synchronous and guard field execution. Property references and
explicitly memoized computation handles have documented, distinct ownership and
cache semantics; see [bindings](PYTHON_BINDINGS.md) and migration examples.

Source part identity comes from OPC relationships rather than fixed filenames.
Output targets are relative to actual owners. Internal graph inconsistencies are
rejected; external targets are not fetched. DOCX/DOTX main-part inputs are accepted
for DOCX output; unsupported main-part profiles/types fail explicitly. Untouched
part payloads retain bytes, but edited XML and the ZIP container are reserialized.
Valid namespace identities and opaque prefix bindings are preserved.

Templates with a package-level OPC digital-signature origin relationship are
rejected before output, even with no explicit edits. rocx does not check
signature validity, strip signatures, or re-sign edited documents. Use an
unsigned copy when you need to edit a signed document.

When reading a part for WordprocessingML editing, rocx rejects unsupported or
unbound namespaces declared mandatory by `mc:MustUnderstand` on its root or
on ordinary WordprocessingML descendants.
Only the baseline Transitional WordprocessingML, Office relationships and XML
namespaces are recognized by this guard. It does not select `mc:AlternateContent`
branches, descend into foreign subtrees, or scan untouched opaque parts.

Text replacement can span runs and operate inside hyperlinks, but does not cross
structured inline boundaries or alter field/revision/control contents. Untouched
break attributes stay intact. Comment/bookmark markers remain at their original
positions; inserted text belongs to the first matched run. This does not promise
universal editing of every Word range structure. Known generated properties are
schema-ordered; unrelated imported children are not blanket-sorted.

See the [template-editing recipe](https://reindervosdewael.github.io/rocx/recipes/#edit-an-existing-document)
for a paired example that saves to a new file.

Compilation completes before path output is replaced using a sibling temporary
file. File reads/writes, clocks and language interoperability live outside the
pure domain/codecs. Python `to_docx()` is an optional python-docx convenience.

## Evidence and limits

For repeatable interactive reader checks, fixture provenance and the current
observation gaps, see the [viewer compatibility checklist](docs/VIEWER-COMPATIBILITY-CHECKLIST.md).
No desktop Word or LibreOffice layout compatibility is established by the
automated checks below.
The checklist also records a limited LibreOffice headless PDF conversion probe;
it is not an interactive open, repair-prompt check, or visual inspection.

Rust/Python expected-content readback checks are complemented by deliberate
mutations, namespace/relocation fixtures, domain-boundary compilation, structured
edit tests, and independent official Transitional XSD checks. The schema validator
also tests invalid order and missing relationships. These checks do not prove
visual equivalence across readers. [Testing instructions](tests/README.md) describe
the exact checks, optional schema cache setup, and wheel verification.

These establish the documented implemented surface and selected conformance
properties, not full OOXML, extension, or visual-layout equivalence across readers.
