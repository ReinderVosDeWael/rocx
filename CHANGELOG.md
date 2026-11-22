# Changelog

## 0.3.0

### Authoring

- Add workbook-free inline column charts with one literal data series and
  stacked inline fractions with two literal text operands. Both are available
  in Rust and Python body paragraphs, including body table cells.
- Set document company and manager, and string and boolean custom properties.
  Template updates preserve unrelated properties and reject attempts to change
  the type of an existing custom property.

### Template safety

- Reject templates declaring an OPC digital-signature origin. The writer does
  not verify, remove, or recreate signatures.
- Reject unsupported mandatory namespaces during supported WordprocessingML
  edits. `AlternateContent` is preserved without choosing a branch.

### Compatibility

The writer emits Transitional DOCX. It does not paginate, render pages, or
evaluate fields. See [the supported feature profile](COMPATIBILITY.md) and
[migration guidance](docs/MIGRATION.md) for the current boundaries.
