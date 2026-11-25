# Changelog

## 0.4.0

### Authoring

- Give each fresh section its own explicitly declared headers and footers.
  Omitting a later story inherits it from the preceding section.
- Size automatic nested tables using the containing cell's grid width,
  including cells that span columns.
- Complete short writes to Python binary streams and report an error if the
  destination stops accepting bytes.

### Existing documents

- Inspect run and hyperlink boundaries in direct body and table-cell paragraphs
  and in default, first-page and even-page headers and footers. Inherited
  references to one physical story share snapshot-scoped inline addresses.
- Combine body alignment, table insertion, and default-header image edits in
  one atomic preview. Retained previews share unchanged package payloads.

### JavaScript

- The `rocx-docx` npm package supports Node and browsers, with named authoring
  factories, reusable snapshots, templates, inspection and edit previews.

### Template fixes

- Preserve paragraph tab-stop definitions when replacing literal tabs in text.
- Reject text replacements that would match across a nonbreaking hyphen or
  another opaque run child with unknown text semantics.
- Reject ordinary replacements in block, row, and cell content controls;
  supported targeted replacements remain available.
- Fill required page margins with built-in defaults when a template lacks
  margin settings, without overwriting existing margins on a partial update.

## 0.3.0

### Authoring

- Add workbook-free inline column charts with one literal data series and
  stacked inline fractions with two literal text operands. Both are available
  in Rust and Python body paragraphs, including body table cells.
- Set document company and manager, and string and boolean custom properties.
  Template updates preserve unrelated properties and reject attempts to change
  the type of an existing custom property.

### Existing documents

- Open Transitional DOCX files from Rust or Python. Inspect body paragraphs,
  sections, direct tables, and default headers; preview supported paragraph
  alignment patches, table insertions, and header image edits. Inline
  inspection does not yet expose editable run or hyperlink boundaries.
- Add a runnable branded-report example in Rust and Python, with a downloadable
  template and output.

### Template safety

- Reject templates declaring an OPC digital-signature origin. The writer does
  not verify, remove, or recreate signatures.
- Reject unsupported mandatory namespaces during supported WordprocessingML
  edits. `AlternateContent` is preserved without choosing a branch.
- Update ZIP and PyO3 to versions that address the advisories listed in the
  [dependency audit](docs/DEPENDENCY-AUDIT.md). Rust 1.88 remains supported.

### Compatibility

The writer emits Transitional DOCX. It does not paginate, render pages, or
evaluate fields. See [the supported feature profile](COMPATIBILITY.md) and
[migration guidance](docs/MIGRATION.md) for the current boundaries.
