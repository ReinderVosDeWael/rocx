# Migrating to 0.2

Version 0.2 redesigns the API. Existing `rv-docx-rs` code will need updates;
old constructor syntax and behavior are not guaranteed.

## Package names

| Former name | Current name |
| --- | --- |
| Rust crate `rv-docx-rs`, imported as `rv_docx_rs` | Crate `rocx`, imported as `rocx` |
| Python distribution `rv-docx-rs` | Distribution `rocx` |
| Python import `rv_docx` | Import `rocx` |

## Rust API

- Paragraphs contain either text or child elements. Use `Paragraph::new` for
  text and `Paragraph::from_children` for runs, links, images, or other content.
- Run and paragraph formatting use `RunFormat` and `ParagraphFormat`.
- Deferred inputs use `Value<T>`. Resolution consumes declarations; resolved
  snapshots can be rendered more than once without rerunning producers.
- Prefer `..Default::default()` when constructing public records with struct
  literals, so optional fields added to the API do not require exhaustive lists.
- If a `ShapeProperties` literal lists every field, add `anchor: None` to
  preserve inline placement. Floating shapes opt in with `with_anchor`.

## Python API

- Import from `rocx` or `rocx.declarative`.
- `await document.resolve()` returns an immutable snapshot and does not mutate
  the declaration. Use factories for async work that must run on each resolution;
  coroutine objects are single-use.
- `Document.save()` and `Document.to_bytes()` resolve automatically.
  `snapshot.to_docx()` is available with the optional `interop` extra.

## Further guidance

See [Getting started](https://reindervosdewael.github.io/rv-docx-rs/getting-started/),
the [feature guides](https://reindervosdewael.github.io/rv-docx-rs/), and the
[compatibility guide](../COMPATIBILITY.md) for examples, supported behavior, and
limitations. The Rust and Python APIs are intentionally similar, but not identical.
