# Migration guide

The documentation site is now at
<https://reindervosdewael.github.io/rocx/>. The former `/rv-docx-rs/`
Pages path returned HTTP 404 when checked; do not rely on a redirect.

This guide separates the legacy `rv-docx-rs` rename/redesign from the
`0.2 → 0.3` upgrade. Existing `rv-docx-rs` code may need updates; old
constructor syntax and behavior are not guaranteed.

## From `rv-docx-rs` to rocx (0.2 redesign)

### Package names

| Former name | Current name |
| --- | --- |
| Rust crate `rv-docx-rs`, imported as `rv_docx_rs` | Crate `rocx`, imported as `rocx` |
| Python distribution `rv-docx-rs` | Distribution `rocx` |
| Python import `rv_docx` | Import `rocx` |

### Rust API

- Paragraphs contain either text or child elements. Use `Paragraph::new` for
  text and `Paragraph::from_children` for runs, links, images, or other content.
- Run and paragraph formatting use `RunFormat` and `ParagraphFormat`.
- Deferred inputs use `Value<T>`. Resolution consumes declarations; resolved
  snapshots can be rendered more than once without rerunning producers.
- Prefer `..Default::default()` when constructing public records with struct
  literals, so optional fields added to the API do not require exhaustive lists.

### Python API

- Import from `rocx` or `rocx.declarative`.
- `await document.resolve()` returns an immutable snapshot and does not mutate
  the declaration. Use factories for async work that must run on each resolution;
  coroutine objects are single-use.
- `Document.save()` and `Document.to_bytes()` resolve automatically.
  `snapshot.to_docx()` is available with the optional `interop` extra.

## From 0.2 to 0.3

- Rust `Metadata` adds `company`, `manager`, `custom_properties`, and
  `custom_boolean_properties`. In exhaustive literals, specify the new fields
  or use `..Default::default()`.
- Template edits now reject signed packages and unsupported mandatory
  namespaces. See the [editing policy](../COMPATIBILITY.md#resolution-editing-and-output)
  and the [0.3.0 changelog](../CHANGELOG.md).

### If your older checkout predates floating shapes

Add `anchor: None` or `..Default::default()` to exhaustive `ShapeProperties`
literals to keep inline placement. Use `with_anchor` to opt into
[floating placement](FORMATTING.md#floating-shapes-and-text-boxes).

## Further guidance

See [Getting started](https://reindervosdewael.github.io/rocx/getting-started/)
for examples and the [compatibility guide](../COMPATIBILITY.md) for supported
behavior and limitations.
