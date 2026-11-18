# Current support and conventions

Version 0.2 follows its own declarative domain model. CMI Docx 0.6.16, revision
`3449b942034129ad74a28aeb253361035806e385`, remains a historical reference for
selected fixtures/tests. Exact CMI constructors and quirks are not requirements.
See [migration](docs/MIGRATION.md), [module boundaries](docs/MODULES.md), and the
[OOXML specification/coverage map](docs/OOXML.md).

## Implemented surface

| Area | Current support |
| --- | --- |
| Declarations | Documents, sections, paragraphs, runs, tables, headers/footers; ready/lazy/async fields and synchronous conditions; deferred Rust metadata/format/property aggregates and immutable resolved-child reuse |
| Dependencies | Rust typed named properties and shared computations; Python per-field references and shared computations; typed graph cycles and cancellation handling |
| Text | Unicode, whitespace, explicit empty runs, tabs, line/page/column breaks |
| Formatting | Selected paragraph/run properties, RGB/theme/automatic run colors, underline styles, explicit false overrides |
| Styles | Paragraph/table definitions and selected conditional table regions, style references and per-compilation catalog |
| Tables | Nested tables, fixed/autofit widths, spans/vertical merges, borders, shading and vertical alignment |
| Sections | Geometry, margins, columns, page numbering, break type, title-page and vertical alignment |
| Stories | Main body, default/first/even headers and footers; comment generation currently restricted to body context |
| Comments | Text/author attached to paragraphs or runs, with paired anchor IDs; not general arbitrary cross-paragraph range authoring |
| Images | PNG/JPEG/GIF/BMP/TIFF metadata and inline pictures, density-aware dimensions and accessibility fields; deferred data/transformation/alt text in both APIs; header inspection without pixel decoding |
| Numbering | Native definitions/levels/references exposed in both Rust and Python |
| Metadata | Core properties and snapshot timestamp |
| Templates | Relationship-driven part discovery, namespace-safe rewriting, opaque payload retention, structural text replacement and insertion |
| Packaging | Typed names/targets/IDs, defaults/overrides, read budgets, internal relationship/reference graph checks and inspectable package plan |

Typed generation for hyperlinks, general fields, footnotes/endnotes, revisions,
content controls, floating drawings, charts, diagrams, equations and broader
Word extensions remains future work. Retaining imported data is not a claim of
typed support. Strict output is explicitly unsupported; the writer uses the
Transitional vocabulary. No Word layout or field-calculation engine is included.

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

Text replacement can span runs and operate inside hyperlinks, but does not cross
structured inline boundaries or alter field/revision/control contents. Untouched
break attributes stay intact. Comment/bookmark markers remain at their original
positions; inserted text belongs to the first matched run. This does not promise
universal editing of every Word range structure. Known generated properties are
schema-ordered; unrelated imported children are not blanket-sorted.

Compilation completes before path output is replaced using a sibling temporary
file. File reads/writes, clocks and language interoperability live outside the
pure domain/codecs. Python `to_docx()` is an optional python-docx convenience.

## Evidence and limits

Rust/Python expected-content readback checks are complemented by deliberate
mutations, namespace/relocation fixtures, domain-boundary compilation, structured
edit tests, and independent official Transitional XSD checks. The schema validator
also tests invalid order and missing relationships. Historical normalized CMI
comparisons preserve useful examples but do not prove schema correctness.
[Testing instructions](tests/README.md) describe the exact checks, optional schema
cache setup, the one retired CMI-only assertion, and wheel verification.

These establish the documented implemented surface and selected conformance
properties, not full OOXML, extension, or visual-layout equivalence across readers.
